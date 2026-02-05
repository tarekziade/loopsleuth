#!/bin/bash
# Regression tests for loopsleuth (Python runner wrapper)

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

python3 "$PROJECT_DIR/tests/run_checks.py" "$@"
