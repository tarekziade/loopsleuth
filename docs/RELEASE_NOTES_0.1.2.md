# LoopSleuth 0.1.2 - Reporting and Check Quality

This release improves report usability, reduces false positives, and makes the test corpus easier to grow.

## What's New

### Reporting
- **HTML reports**: `--output` now writes HTML with clearer structure and styling.
- **Hotspot highlighting**: Suspected lines are highlighted with a light red background for quick review.
- **Verbose clarity**: When a solution is rejected, verbose output now explains why.

### Detection Quality
- **Configurable deduping**: Check result deduplication is now driven by TOML rules.
- **Stricter guards**: New guard rules reduce false positives for several ML checks.
- **Solution validation messaging**: “No safe change suggested” when a solution matches the original or cannot be extracted.

### Test Corpus
- **Tests reorganized**: `test_examples/` is now `tests/` with one file per check.
- **Golden files**: Standardized golden outputs for deterministic verification.
- **Makefile updates**: New targets for test runs and golden updates.

## Upgrade Notes

- Reports saved with `-o` are now HTML. Rename output files with `.html` for best viewing.
- Existing TOML configs remain compatible; new optional dedupe/guard settings are available.

## Installation

```bash
pip install --upgrade loopsleuth
```

## Contributors

- Tarek Ziadé (@tarekziade)

---

For full documentation, see the README.
