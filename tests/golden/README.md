# Golden Files

This directory stores golden expectations for per-check examples.

Generate or update goldens after changing prompts or example files:

```bash
python3 tests/run_checks.py --update-golden
```

Golden files are keyed by check name (for example: `quadratic.json`) and store expected issue names and solution paths.
