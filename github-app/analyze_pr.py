#!/usr/bin/env python3
"""LoopSleuth GitHub Action — analyze changed Python functions in a PR.

Runs as a single script with no extra dependencies beyond loopsleuth.
Uses the GitHub API via `gh` CLI (pre-installed on all GitHub runners).
"""

import ast
import datetime
import functools
import json
import os
import re
import subprocess
import sys
import tempfile
from pathlib import Path

# Force unbuffered output so CI streams logs in real time
print = functools.partial(print, flush=True)  # noqa: A001

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

def find_changed_functions(source: str, changed_lines: list[int]) -> dict[str, tuple[int, int]]:
    """Return {name: (start_line, end_line)} for functions overlapping changed lines."""
    try:
        tree = ast.parse(source)
    except SyntaxError:
        return {}

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

    return {
        name: (start, end)
        for name, (start, end) in functions.items()
        if any(start <= line <= end for line in changed_lines)
    }


def extract_functions_source(source: str, func_ranges: dict[str, tuple[int, int]]) -> str:
    """Extract only the specified functions from source, producing a valid Python file."""
    lines = source.splitlines(keepends=True)
    # Collect all imports at the top (so extracted functions can reference them)
    import_lines = []
    for line in lines:
        stripped = line.strip()
        if stripped.startswith(("import ", "from ")) or stripped == "":
            import_lines.append(line)
        elif stripped.startswith(("#", '"""', "'''")):
            continue
        else:
            break

    # Extract each function's lines
    func_blocks = []
    for _name, (start, end) in sorted(func_ranges.items(), key=lambda x: x[1][0]):
        func_blocks.append("".join(lines[start - 1:end]))
        func_blocks.append("\n\n")

    return "".join(import_lines) + "\n" + "".join(func_blocks)


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

    print(f"    Running: {' '.join(cmd)}", flush=True)

    # Stream stderr live so CI shows progress; capture stdout for JSON
    proc = subprocess.Popen(
        cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True,
    )

    # Read stderr in a thread so it streams while stdout accumulates
    import threading

    stderr_lines: list[str] = []

    def stream_stderr():
        for line in proc.stderr:
            line = line.rstrip()
            stderr_lines.append(line)
            print(f"    [loopsleuth] {line}", flush=True)

    t = threading.Thread(target=stream_stderr, daemon=True)
    t.start()

    stdout = proc.stdout.read()
    proc.wait(timeout=600)
    t.join(timeout=5)

    print(f"    Exit code: {proc.returncode}", flush=True)
    if proc.returncode != 0:
        print(f"::warning::loopsleuth failed on {file_path}: {' '.join(stderr_lines)[-500:]}")
        return {}
    print(f"    Stdout length: {len(stdout)} chars", flush=True)
    try:
        data = json.loads(stdout)
        n_funcs = data.get("total_functions", 0)
        n_issues = data.get("functions_with_issues", 0)
        print(f"    Result: {n_funcs} function(s) analyzed, {n_issues} with issues", flush=True)
        return data
    except json.JSONDecodeError:
        print(f"::warning::Invalid JSON from loopsleuth for {file_path}")
        print(f"    Raw stdout: {stdout[:300]}", flush=True)
        return {}


# ---------------------------------------------------------------------------
# 6. Format the PR comment
# ---------------------------------------------------------------------------

def _run_url() -> str:
    """Build a link to the current GitHub Actions run."""
    server = os.environ.get("GITHUB_SERVER_URL", "https://github.com")
    repo = os.environ.get("GITHUB_REPOSITORY", "")
    run_id = os.environ.get("GITHUB_RUN_ID", "")
    if repo and run_id:
        return f"{server}/{repo}/actions/runs/{run_id}"
    return ""


def format_comment(issues: list[dict], total_tokens: int = 0, model: str = "", head_sha: str = "", repo: str = "") -> str:
    total_issues = sum(
        len(f["issues"]) for file in issues for f in file["results"]
    )
    total_funcs = sum(len(file["results"]) for file in issues)
    total_files = len(issues)

    now = datetime.datetime.utcnow().strftime("%Y-%m-%d %H:%M UTC")

    lines = [
        COMMENT_MARKER,
        "## :x: LoopSleuth Performance Analysis",
        "",
        f"Found **{total_issues} {'issue' if total_issues == 1 else 'issues'}** in "
        f"**{total_funcs} {'function' if total_funcs == 1 else 'functions'}** across "
        f"**{total_files} {'file' if total_files == 1 else 'files'}**.",
        "",
    ]

    server = os.environ.get("GITHUB_SERVER_URL", "https://github.com")

    for file in issues:
        path = file["path"]
        n = sum(len(f["issues"]) for f in file["results"])
        file_url = f"{server}/{repo}/blob/{head_sha}/{path}" if repo and head_sha else ""
        lines.append(
            f"<details>\n<summary><b>{path}</b> &mdash; {n} {'issue' if n == 1 else 'issues'}</summary>\n"
        )

        for func in file["results"]:
            name = func["function_name"]
            cls = func.get("class_name")
            display = f"{cls}.{name}" if cls else name
            line_num = func.get("line_number", "?")
            line_url = f"{file_url}#L{line_num}" if file_url and line_num != "?" else ""
            loc = f"[line {line_num}]({line_url})" if line_url else f"line {line_num}"

            for issue in func["issues"]:
                check = issue.get("check_name", issue.get("check_key", "?"))
                confidence = issue.get("confidence", 0)
                solution = issue.get("solution")

                lines.append(f"### [`{display}` ({path}:{line_num})]({line_url})" if line_url else f"### `{display}` ({path}:{line_num})")
                lines.append("")
                lines.append(f"**{check}** (confidence: {confidence}%)")
                lines.append("")

                # Show the DETAIL line from the analysis
                analysis = issue.get("analysis", "")
                for aline in analysis.splitlines():
                    if aline.strip().startswith("DETAIL:"):
                        detail = aline.strip()[7:].strip()
                        if detail:
                            lines.append(f"> {detail}")
                            lines.append("")
                        break

                if solution:
                    lines.append("<details>\n<summary>Suggested fix</summary>\n")
                    lines.append(solution)
                    lines.append("\n</details>\n")
                else:
                    lines.append("*No safe fix suggested — consider reviewing manually.*\n")

        lines.append("</details>\n")

    lines.append("---")
    lines.append(_format_footer(now, total_tokens, model))
    return "\n".join(lines)


def format_clean_comment(total_tokens: int = 0, model: str = "") -> str:
    now = datetime.datetime.utcnow().strftime("%Y-%m-%d %H:%M UTC")
    return (
        f"{COMMENT_MARKER}\n"
        "## :white_check_mark: LoopSleuth Performance Analysis\n\n"
        "No performance issues detected in the changed Python functions.\n\n"
        "---\n"
        + _format_footer(now, total_tokens, model)
    )


def _format_footer(now: str, total_tokens: int, model: str) -> str:
    parts = [f"[LoopSleuth](https://github.com/tarekziade/loopsleuth)"]
    if model:
        parts.append(f"model: `{model}`")
    if total_tokens:
        parts.append(f"{total_tokens:,} tokens")
    run = _run_url()
    if run:
        parts.append(f"[full log]({run})")
    return f"*{' | '.join(parts)} | {now}*"


# ---------------------------------------------------------------------------
# 7. Post or update the PR comment
# ---------------------------------------------------------------------------

def post_or_update_comment(pr_number: int, body: str) -> None:
    # Write body to a temp file to avoid shell escaping issues
    body_file = Path(tempfile.mktemp(suffix=".md"))
    body_file.write_text(body)

    # For API PATCH, we need a JSON file
    json_file = Path(tempfile.mktemp(suffix=".json"))
    json_file.write_text(json.dumps({"body": body}))

    try:
        # Look for an existing LoopSleuth comment to update
        repo = os.environ.get("GITHUB_REPOSITORY", "")
        comments_raw = gh(
            "api", f"/repos/{repo}/issues/{pr_number}/comments",
            "--paginate", "-q", ".[].id",
        )
        for comment_id in comments_raw.strip().splitlines():
            comment_id = comment_id.strip()
            if not comment_id:
                continue
            comment_body = gh(
                "api", f"/repos/{repo}/issues/comments/{comment_id}",
                "-q", ".body",
            )
            if COMMENT_MARKER in comment_body:
                gh(
                    "api", f"/repos/{repo}/issues/comments/{comment_id}",
                    "-X", "PATCH",
                    "--input", str(json_file),
                )
                print(f"Updated existing comment {comment_id} on PR #{pr_number}")
                return

        # No existing comment — create new one
        gh("pr", "comment", str(pr_number), "--body-file", str(body_file))
        print(f"Posted new comment on PR #{pr_number}")
    finally:
        body_file.unlink(missing_ok=True)
        json_file.unlink(missing_ok=True)


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------

def main():
    pr = get_pr_info()
    pr_number = pr["number"]
    print(f"Analyzing PR #{pr_number} on {pr['repo']}")

    # Check loopsleuth version
    try:
        ver = subprocess.run(["loopsleuth", "--help"], capture_output=True, text=True)
        has_api = "--api-url" in ver.stdout
        has_json = "--format" in ver.stdout
        print(f"  loopsleuth installed: --api-url={'yes' if has_api else 'NO'}, --format={'yes' if has_json else 'NO'}")
    except FileNotFoundError:
        print("::error::loopsleuth not found in PATH")
        sys.exit(1)

    # Get changed Python files
    changed_files = get_changed_py_files(pr_number)
    if not changed_files:
        print("No Python files changed — skipping")
        return

    print(f"Found {len(changed_files)} changed Python file(s)")

    all_issues: list[dict] = []
    total_input_tokens = 0
    total_output_tokens = 0
    model_name = ""

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
            print(f"  {rel_path}: {len(changed_lines)} changed line(s)")
            changed_funcs = find_changed_functions(content, changed_lines)

            if not changed_funcs:
                print(f"  {rel_path}: no functions overlap with changed lines, skipping")
                continue

            print(f"  {rel_path}: {len(changed_funcs)} function(s) changed: {', '.join(changed_funcs.keys())}")

            # Write ONLY the changed functions to the temp file
            tmp_file = Path(tmpdir) / rel_path
            tmp_file.parent.mkdir(parents=True, exist_ok=True)
            extracted = extract_functions_source(content, changed_funcs)
            tmp_file.write_text(extracted)
            print(f"    Extracted {len(changed_funcs)} function(s) to temp file ({len(extracted)} chars)")

            result = run_loopsleuth(str(tmp_file))
            if not result:
                continue

            usage = result.get("token_usage", {})
            total_input_tokens += usage.get("input_tokens", 0)
            total_output_tokens += usage.get("output_tokens", 0)
            if not model_name:
                model_name = result.get("model", "")

            # Collect issues (all results are relevant since we only gave it changed functions)
            file_issues = []
            for file_result in result.get("files", []):
                for func_result in file_result.get("results", []):
                    func_name = func_result.get("function_name", "")
                    class_name = func_result.get("class_name")
                    qualified = f"{class_name}.{func_name}" if class_name else func_name

                    issues = func_result.get("issues", [])
                    if issues:
                        for iss in issues:
                            print(f"    ISSUE in {qualified}: {iss.get('check_name')} (confidence: {iss.get('confidence')}%)")
                        # Use original line number from the full file
                        original_line = changed_funcs.get(func_name, changed_funcs.get(qualified, (0, 0)))[0]
                        file_issues.append({
                            "function_name": func_name,
                            "class_name": class_name,
                            "line_number": original_line,
                            "issues": issues,
                        })
                    else:
                        print(f"    {qualified}: clean")

            if file_issues:
                all_issues.append({"path": rel_path, "results": file_issues})

    total_tokens = total_input_tokens + total_output_tokens

    # Post comment only if issues found; fail the run so PR shows red
    if all_issues:
        total = sum(len(f["issues"]) for file in all_issues for f in file["results"])
        print(f"\nFound {total} issue(s) — posting comment ({total_tokens} tokens, model: {model_name})")
        body = format_comment(all_issues, total_tokens, model_name, pr["head_sha"], pr["repo"])
        post_or_update_comment(pr_number, body)
        sys.exit(1)  # fail the check so the PR status is red
    else:
        print(f"\nNo performance issues found ({total_tokens} tokens, model: {model_name})")


if __name__ == "__main__":
    main()
