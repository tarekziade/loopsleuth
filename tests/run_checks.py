#!/usr/bin/env python3
"""Run per-check examples and compare against golden files."""

from __future__ import annotations

import argparse
import ast
import json
import os
import re
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass
from pathlib import Path
from typing import Dict, List, Optional, Tuple


ROOT = Path(__file__).resolve().parents[1]
CHECKS_DIR = ROOT / "tests" / "checks"
GOLDEN_DIR = ROOT / "tests" / "golden"


@dataclass
class IssueResult:
    issue_name: str
    solution: Optional[str]


def default_model_path() -> str:
    env_model = os.environ.get("LOOPSLEUTH_TEST_MODEL")
    if env_model:
        return env_model
    return str(Path.home() / ".loopsleuth" / "models" / "Qwen2.5-Coder-7B-Instruct-Q4_K_M.gguf")


def normalize_code(code: str) -> str:
    lines = [line.rstrip() for line in code.strip().splitlines()]
    # Remove leading/trailing empty lines after stripping
    while lines and lines[0] == "":
        lines.pop(0)
    while lines and lines[-1] == "":
        lines.pop()
    return "\n".join(lines)


def check_key_from_filename(path: Path) -> str:
    return path.stem.replace("_", "-")


def list_functions(path: Path) -> List[str]:
    tree = ast.parse(path.read_text())
    funcs: List[str] = []

    for node in tree.body:
        if isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            funcs.append(node.name)
        elif isinstance(node, ast.ClassDef):
            for item in node.body:
                if isinstance(item, (ast.FunctionDef, ast.AsyncFunctionDef)):
                    funcs.append(f"{node.name}::{item.name}")

    return funcs


def run_loopsleuth(binary: Path, model: str, config: Optional[str], check_key: str, file_path: Path) -> Path:
    if not binary.exists():
        raise FileNotFoundError(f"Binary not found: {binary}")
    if not Path(model).exists():
        raise FileNotFoundError(f"Model not found: {model}")

    report_fd, report_path = tempfile.mkstemp(prefix=f"loopsleuth_{check_key}_", suffix=".md")
    os.close(report_fd)
    report_file = Path(report_path)

    cmd = [
        str(binary),
        str(file_path),
        "-m",
        model,
        "--checks",
        check_key,
        "--details",
        "--output",
        str(report_file),
        "--no-cache",
    ]
    if config:
        cmd.extend(["--config", config])

    print(f"Running: {' '.join(cmd)}", flush=True)
    start = time.time()
    proc = subprocess.Popen(cmd, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)

    while proc.poll() is None:
        elapsed = int(time.time() - start)
        if elapsed > 0 and elapsed % 5 == 0:
            print(f"... still running ({elapsed}s)", flush=True)
        time.sleep(1)

    stdout, stderr = proc.communicate()
    if proc.returncode != 0:
        raise RuntimeError(
            "Command failed:\n"
            f"{' '.join(cmd)}\n"
            f"stdout:\n{stdout}\n"
            f"stderr:\n{stderr}"
        )

    return report_file


def parse_report(report_path: Path) -> Dict[str, IssueResult]:
    issues: Dict[str, IssueResult] = {}
    current_func: Optional[str] = None
    current_issue: Optional[str] = None
    in_solution = False
    solution_lines: List[str] = []

    issue_re = re.compile(r"^#### .*Issue(?: \d+)?: (.+?) \(confidence:")
    func_re = re.compile(r"^### \d+ - `(.+?)`$")
    awaiting_solution = False

    for raw_line in report_path.read_text().splitlines():
        line = raw_line.rstrip()

        func_match = func_re.match(line)
        if func_match:
            if in_solution and current_func and current_issue:
                issues[current_func] = IssueResult(current_issue, "\n".join(solution_lines))
            current_func = func_match.group(1)
            current_issue = None
            in_solution = False
            solution_lines = []
            continue

        issue_match = issue_re.match(line)
        if issue_match:
            current_issue = issue_match.group(1)
            continue

        if "Suggested Optimization" in line:
            awaiting_solution = True
            in_solution = False
            solution_lines = []
            continue

        if line.strip() == "```python" and awaiting_solution:
            in_solution = True
            solution_lines = []
            awaiting_solution = False
            continue

        if in_solution and line.strip() == "```":
            in_solution = False
            if current_func and current_issue:
                issues[current_func] = IssueResult(current_issue, "\n".join(solution_lines))
            solution_lines = []
            continue

        if in_solution:
            solution_lines.append(raw_line)

    if in_solution and current_func and current_issue:
        issues[current_func] = IssueResult(current_issue, "\n".join(solution_lines))

    return issues


def load_golden(check_key: str) -> Dict:
    golden_path = GOLDEN_DIR / f"{check_key}.json"
    if not golden_path.exists():
        raise FileNotFoundError(f"Golden file missing: {golden_path}")
    return json.loads(golden_path.read_text())


def write_golden(check_key: str, issues: Dict[str, IssueResult], all_functions: List[str]) -> None:
    check_dir = GOLDEN_DIR / check_key
    check_dir.mkdir(parents=True, exist_ok=True)

    issues_out: Dict[str, Dict[str, Optional[str]]] = {}
    for func_name, result in sorted(issues.items()):
        solution_path = None
        if result.solution:
            file_path = check_dir / f"{func_name}.py"
            file_path.write_text(normalize_code(result.solution) + "\n")
            solution_path = str(file_path.relative_to(ROOT))
        issues_out[func_name] = {
            "issue": result.issue_name,
            "solution_path": solution_path,
        }

    clean = [name for name in all_functions if name not in issues]

    golden = {
        "check_key": check_key,
        "issues": issues_out,
        "clean": sorted(clean),
    }

    GOLDEN_DIR.mkdir(parents=True, exist_ok=True)
    (GOLDEN_DIR / f"{check_key}.json").write_text(json.dumps(golden, indent=2) + "\n")


def verify_golden(check_key: str, issues: Dict[str, IssueResult], all_functions: List[str]) -> Tuple[bool, List[str]]:
    errors: List[str] = []
    golden = load_golden(check_key)

    expected_issues = golden.get("issues", {})
    expected_clean = set(golden.get("clean", []))

    actual_issue_names = set(issues.keys())
    expected_issue_names = set(expected_issues.keys())

    missing = expected_issue_names - actual_issue_names
    extra = actual_issue_names - expected_issue_names

    if missing:
        errors.append(f"Missing issues: {sorted(missing)}")
    if extra:
        errors.append(f"Unexpected issues: {sorted(extra)}")

    for func_name in expected_issue_names & actual_issue_names:
        expected_issue = expected_issues[func_name].get("issue")
        actual_issue = issues[func_name].issue_name
        if expected_issue != actual_issue:
            errors.append(f"Issue name mismatch for {func_name}: expected '{expected_issue}', got '{actual_issue}'")

        expected_solution_path = expected_issues[func_name].get("solution_path")
        actual_solution = issues[func_name].solution
        if expected_solution_path:
            solution_path = Path(expected_solution_path)
            if not solution_path.is_absolute():
                solution_path = ROOT / solution_path
            expected_solution = solution_path.read_text()
            if actual_solution is None:
                errors.append(f"Missing solution for {func_name}")
            else:
                if normalize_code(actual_solution) != normalize_code(expected_solution):
                    errors.append(f"Solution mismatch for {func_name}")
        else:
            if actual_solution:
                errors.append(f"Unexpected solution for {func_name}")

    all_function_set = set(all_functions)
    if expected_clean and not expected_clean.issubset(all_function_set):
        errors.append("Golden clean list contains unknown functions")

    unexpected_flagged = actual_issue_names & expected_clean
    if unexpected_flagged:
        errors.append(f"Expected clean but flagged: {sorted(unexpected_flagged)}")

    covered = expected_issue_names | expected_clean
    uncovered = all_function_set - covered
    if uncovered:
        errors.append(f"Uncovered functions (missing from golden): {sorted(uncovered)}")

    return len(errors) == 0, errors


def main() -> int:
    parser = argparse.ArgumentParser(description="Run LoopSleuth check examples against golden files.")
    parser.add_argument("--binary", default=str(ROOT / "target" / "release" / "loopsleuth_bin"))
    parser.add_argument("--model", default=default_model_path())
    parser.add_argument("--config", default=str(ROOT / "loopsleuth.toml"))
    parser.add_argument("--checks", help="Comma-separated list of checks to run")
    parser.add_argument("--update-golden", action="store_true")
    args = parser.parse_args()

    checks = sorted(CHECKS_DIR.glob("*.py"))
    if not checks:
        print(f"No checks found under {CHECKS_DIR}")
        return 1

    if args.checks:
        allowed = {name.strip() for name in args.checks.split(",") if name.strip()}
        checks = [c for c in checks if check_key_from_filename(c) in allowed]

    failures = 0
    for check_path in checks:
        check_key = check_key_from_filename(check_path)
        print(f"\n== {check_key} ==")
        all_functions = list_functions(check_path)
        report_path = run_loopsleuth(Path(args.binary), args.model, args.config, check_key, check_path)
        issues = parse_report(report_path)
        report_path.unlink(missing_ok=True)

        if args.update_golden:
            write_golden(check_key, issues, all_functions)
            print("Updated golden file.")
        else:
            try:
                ok, errors = verify_golden(check_key, issues, all_functions)
            except FileNotFoundError as exc:
                failures += 1
                print(f"- {exc}")
                print("- Run with --update-golden to generate expected outputs")
                continue

            if ok:
                print("OK")
            else:
                failures += 1
                for error in errors:
                    print(f"- {error}")

    return 1 if failures else 0


if __name__ == "__main__":
    raise SystemExit(main())
