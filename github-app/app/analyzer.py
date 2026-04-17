"""Run loopsleuth CLI and parse JSON results."""

import json
import os
import subprocess
import tempfile
from pathlib import Path

from .config import settings


def run_loopsleuth(file_path: str) -> dict:
    """Run loopsleuth against a single file and return parsed JSON output."""
    env = os.environ.copy()
    env["HF_TOKEN"] = settings.HF_TOKEN

    result = subprocess.run(
        [
            "loopsleuth",
            "--api-url", settings.LOOPSLEUTH_API_URL,
            "--no-cache",
            "--format", "json",
            file_path,
        ],
        capture_output=True,
        text=True,
        env=env,
        timeout=600,
    )

    if result.returncode != 0:
        return {"error": result.stderr.strip(), "files": []}

    try:
        return json.loads(result.stdout)
    except json.JSONDecodeError:
        return {"error": f"Invalid JSON: {result.stdout[:200]}", "files": []}


def analyze_files(
    files: dict[str, str],
    changed_functions: dict[str, set[str]],
) -> list[dict]:
    """Analyze multiple files. Returns list of per-file results with issues.

    Args:
        files: {relative_path: file_content}
        changed_functions: {relative_path: {func_names}}
    """
    all_issues = []

    with tempfile.TemporaryDirectory(prefix="loopsleuth_") as tmpdir:
        for rel_path, content in files.items():
            tmp_file = Path(tmpdir) / rel_path
            tmp_file.parent.mkdir(parents=True, exist_ok=True)
            tmp_file.write_text(content)

            result = run_loopsleuth(str(tmp_file))

            if "error" in result and result["error"]:
                continue

            funcs_to_report = changed_functions.get(rel_path, set())

            for file_result in result.get("files", []):
                file_issues = []
                for func_result in file_result.get("results", []):
                    func_name = func_result.get("function_name", "")
                    class_name = func_result.get("class_name")
                    qualified = f"{class_name}.{func_name}" if class_name else func_name

                    # Only report functions that were actually changed in the PR
                    if not _matches_any(qualified, func_name, funcs_to_report):
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
                    all_issues.append({
                        "path": rel_path,
                        "results": file_issues,
                    })

    return all_issues


def _matches_any(qualified: str, short: str, targets: set[str]) -> bool:
    """Check if a function name matches any target (qualified or short)."""
    if not targets:
        return True  # no filter, report all
    return qualified in targets or short in targets
