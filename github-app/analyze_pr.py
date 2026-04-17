#!/usr/bin/env python3
"""LoopSleuth GitHub Action — analyze changed Python functions in a PR.

Runs as a single script with no extra dependencies beyond loopsleuth.
Uses the GitHub API via `gh` CLI (pre-installed on all GitHub runners).
"""

import ast
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

# ---------------------------------------------------------------------------
# Config
# ---------------------------------------------------------------------------

API_URL = os.environ["LOOPSLEUTH_API_URL"]
MAX_FILES = int(os.environ.get("LOOPSLEUTH_MAX_FILES", "15"))
CHECKS = os.environ.get("LOOPSLEUTH_CHECKS", "")
COMMENT_MARKER = "<!-- loopsleuth-report -->"


def gh(*args: str) -> str:
    """Run a gh CLI command and return stdout."""
    result = subprocess.run(
        ["gh", *args],
        capture_output=True, text=True, check=True,
    )
    return result.stdout


# ---------------------------------------------------------------------------
# 1. Get PR context from the GitHub event
# ---------------------------------------------------------------------------

def get_pr_info() -> dict:
    event_path = os.environ.get("GITHUB_EVENT_PATH")
    if not event_path:
        sys.exit("GITHUB_EVENT_PATH not set — must run inside a GitHub Action")
    with open(event_path) as f:
        event = json.load(f)
    pr = event.get("pull_request")
    if not pr:
        sys.exit("Not a pull_request event")
    return {
        "number": pr["number"],
        "head_sha": pr["head"]["sha"],
        "repo": event["repository"]["full_name"],
    }


# ---------------------------------------------------------------------------
# 2. Fetch changed Python files
# ---------------------------------------------------------------------------

def get_changed_py_files(pr_number: int) -> list[dict]:
    """Use gh CLI to get changed files with their patches."""
    raw = gh(
        "pr", "diff", str(pr_number),
    )
    # Also get the file list
    files_json = gh(
        "pr", "view", str(pr_number),
        "--json", "files",
    )
    files = json.loads(files_json).get("files", [])

    py_files = []
    for f in files:
        path = f.get("path", "")
        if not path.endswith(".py"):
            continue
        if f.get("deletions", 0) > 0 or f.get("additions", 0) > 0:
            py_files.append(path)

    if len(py_files) > MAX_FILES:
        print(f"::warning::Limiting analysis to {MAX_FILES} of {len(py_files)} Python files")
        py_files = py_files[:MAX_FILES]

    # Parse the unified diff to get per-file patches
    result = []
    for path in py_files:
        patch = extract_file_patch(raw, path)
        result.append({"path": path, "patch": patch})

    return result


def extract_file_patch(full_diff: str, filename: str) -> str:
    """Extract the patch for a specific file from a full unified diff."""
    lines = full_diff.splitlines(keepends=True)
    in_file = False
    patch_lines = []

    for line in lines:
        if line.startswith("diff --git"):
            if in_file:
                break  # reached next file
            # Check if this is our file
            if f"b/{filename}" in line:
                in_file = True
            continue
        if in_file:
            # Skip the diff header lines (---, +++, index)
            if line.startswith("index ") or line.startswith("new file"):
                continue
            patch_lines.append(line)

    return "".join(patch_lines)


# ---------------------------------------------------------------------------
# 3. Diff parser — extract changed line numbers
# ---------------------------------------------------------------------------

def extract_changed_lines(patch: str) -> list[int]:
    changed = []
    current_line = 0
    for line in patch.splitlines():
        hunk = re.match(r"^@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@", line)
        if hunk:
            current_line = int(hunk.group(1))
            continue
        if line.startswith("+") and not line.startswith("+++"):
            changed.append(current_line)
            current_line += 1
        elif line.startswith("-") and not line.startswith("---"):
            pass
        else:
            current_line += 1
    return changed


# ---------------------------------------------------------------------------
# 4. Function extractor — map lines to function names via AST
# ---------------------------------------------------------------------------

def find_changed_functions(source: str, changed_lines: list[int]) -> set[str]:
    try:
        tree = ast.parse(source)
    except SyntaxError:
        return set()

    functions: dict[str, tuple[int, int]] = {}
    for node in ast.walk(tree):
        if isinstance(node, ast.ClassDef):
            for item in node.body:
                if isinstance(item, (ast.FunctionDef, ast.AsyncFunctionDef)):
                    name = f"{node.name}.{item.name}"
                    functions[name] = (item.lineno, item.end_lineno or item.lineno)
        elif isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            if node.col_offset == 0:
                functions[node.name] = (node.lineno, node.end_lineno or node.lineno)

    result = set()
    for name, (start, end) in functions.items():
        if any(start <= line <= end for line in changed_lines):
            result.add(name)
    return result


# ---------------------------------------------------------------------------
# 5. Run loopsleuth
# ---------------------------------------------------------------------------

def run_loopsleuth(file_path: str) -> dict:
    cmd = [
        "loopsleuth",
        "--api-url", API_URL,
        "--no-cache",
        "--format", "json",
    ]
    if CHECKS:
        cmd.extend(["--checks", CHECKS])
    cmd.append(file_path)

    result = subprocess.run(
        cmd, capture_output=True, text=True, timeout=600,
    )
    if result.returncode != 0:
        print(f"::warning::loopsleuth failed on {file_path}: {result.stderr.strip()}")
        return {}
    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError:
        print(f"::warning::Invalid JSON from loopsleuth for {file_path}")
        return {}


# ---------------------------------------------------------------------------
# 6. Format the PR comment
# ---------------------------------------------------------------------------

def format_comment(issues: list[dict]) -> str:
    total_issues = sum(
        len(f["issues"]) for file in issues for f in file["results"]
    )
    total_funcs = sum(len(file["results"]) for file in issues)
    total_files = len(issues)

    lines = [
        COMMENT_MARKER,
        "## LoopSleuth Performance Analysis",
        "",
        f"Found **{total_issues} issue(s)** in "
        f"**{total_funcs} function(s)** across "
        f"**{total_files} file(s)**.",
        "",
    ]

    for file in issues:
        path = file["path"]
        n = sum(len(f["issues"]) for f in file["results"])
        lines.append(
            f"<details>\n<summary><b>{path}</b> &mdash; {n} issue(s)</summary>\n"
        )

        for func in file["results"]:
            name = func["function_name"]
            cls = func.get("class_name")
            display = f"{cls}.{name}" if cls else name
            line_num = func.get("line_number", "?")

            for issue in func["issues"]:
                check = issue.get("check_name", issue.get("check_key", "?"))
                confidence = issue.get("confidence", 0)
                solution = issue.get("solution")

                lines.append(f"### `{display}` (line {line_num})")
                lines.append("")
                lines.append(f"**{check}** (confidence: {confidence}%)")
                lines.append("")

                if solution:
                    lines.append("<details>\n<summary>Suggested fix</summary>\n")
                    lines.append(solution)
                    lines.append("\n</details>\n")

        lines.append("</details>\n")

    lines.append("---")
    lines.append(
        "*Generated by [LoopSleuth](https://github.com/tarekziade/loopsleuth)*"
    )
    return "\n".join(lines)


def format_clean_comment() -> str:
    return (
        f"{COMMENT_MARKER}\n"
        "## LoopSleuth Performance Analysis\n\n"
        "No performance issues detected in the changed Python functions.\n\n"
        "---\n"
        "*Generated by [LoopSleuth](https://github.com/tarekziade/loopsleuth)*"
    )


# ---------------------------------------------------------------------------
# 7. Post or update the PR comment
# ---------------------------------------------------------------------------

def post_or_update_comment(pr_number: int, body: str) -> None:
    # List existing comments and look for our marker
    comments_json = gh(
        "pr", "view", str(pr_number),
        "--json", "comments",
    )
    comments = json.loads(comments_json).get("comments", [])

    for c in comments:
        if COMMENT_MARKER in (c.get("body") or ""):
            # Update existing comment
            comment_url = c.get("url", "")
            # Extract comment ID from the URL or use the API
            if comment_url:
                gh("api", comment_url, "-X", "PATCH", "-f", f"body={body}")
                print(f"Updated existing comment on PR #{pr_number}")
                return

    # No existing comment — create new one
    gh("pr", "comment", str(pr_number), "--body", body)
    print(f"Posted new comment on PR #{pr_number}")


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    pr = get_pr_info()
    pr_number = pr["number"]
    print(f"Analyzing PR #{pr_number} on {pr['repo']}")

    # Get changed Python files
    changed_files = get_changed_py_files(pr_number)
    if not changed_files:
        print("No Python files changed — skipping")
        return

    print(f"Found {len(changed_files)} changed Python file(s)")

    all_issues: list[dict] = []

    with tempfile.TemporaryDirectory(prefix="loopsleuth_") as tmpdir:
        for cf in changed_files:
            rel_path = cf["path"]
            patch = cf["patch"]

            # Fetch file content at HEAD
            try:
                content = gh(
                    "api",
                    f"/repos/{pr['repo']}/contents/{rel_path}",
                    "-q", ".content",
                    "--header", f"Accept: application/vnd.github.v3+json",
                    "-X", "GET",
                    "-f", f"ref={pr['head_sha']}",
                )
                import base64
                content = base64.b64decode(content).decode("utf-8")
            except Exception as e:
                print(f"::warning::Could not fetch {rel_path}: {e}")
                continue

            # Find which functions were changed
            changed_lines = extract_changed_lines(patch)
            changed_funcs = find_changed_functions(content, changed_lines)

            if not changed_funcs:
                print(f"  {rel_path}: no functions changed, skipping")
                continue

            print(f"  {rel_path}: {len(changed_funcs)} function(s) changed: {', '.join(changed_funcs)}")

            # Write to temp and run loopsleuth
            tmp_file = Path(tmpdir) / rel_path
            tmp_file.parent.mkdir(parents=True, exist_ok=True)
            tmp_file.write_text(content)

            result = run_loopsleuth(str(tmp_file))
            if not result:
                continue

            # Filter results to only changed functions
            file_issues = []
            for file_result in result.get("files", []):
                for func_result in file_result.get("results", []):
                    func_name = func_result.get("function_name", "")
                    class_name = func_result.get("class_name")
                    qualified = f"{class_name}.{func_name}" if class_name else func_name

                    if func_name not in changed_funcs and qualified not in changed_funcs:
                        continue

                    issues = func_result.get("issues", [])
                    if issues:
                        file_issues.append({
                            "function_name": func_name,
                            "class_name": class_name,
                            "line_number": func_result.get("line_number", 0),
                            "issues": issues,
                        })

            if file_issues:
                all_issues.append({"path": rel_path, "results": file_issues})

    # Post comment
    if all_issues:
        total = sum(len(f["issues"]) for file in all_issues for f in file["results"])
        print(f"\nFound {total} issue(s) — posting comment")
        body = format_comment(all_issues)
    else:
        print("\nNo issues found — posting clean report")
        body = format_clean_comment()

    post_or_update_comment(pr_number, body)


if __name__ == "__main__":
    main()
