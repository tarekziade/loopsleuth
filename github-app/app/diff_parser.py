"""Parse unified diffs to extract changed line numbers."""

import re


def extract_changed_lines(patch: str) -> list[int]:
    """Extract new-side line numbers that were added or modified."""
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
            pass  # deleted line, don't advance new-side counter
        else:
            current_line += 1
    return changed
