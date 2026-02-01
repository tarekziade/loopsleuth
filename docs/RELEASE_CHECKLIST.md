# How to release 

## Pre-Release Checklist

- [ ] All tests pass locally
- [ ] Install works locally, `loopsleuth --help` works
- [ ] Documentation is up-to-date
- [ ] All example commands in README.md work correctly
- [ ] Version numbers match across all files (0.1.0)
- [ ] All commits are pushed to the main branch
- [ ] No untracked files or uncommitted changes


## Release Process

### 0. bump version in `pyproject.toml`, respect semantic versioning (you can ask if its minor or patch)

### 1. Create a Git Tag
```bash
# Make sure all changes are committed
git status

# Create and push the version tag
git tag -a v0.1.0 -m "Release version 0.1.0"
git push origin v0.1.0
```

### 2. Create GitHub Release

- [ ] check in git logs all changes since last release
- [ ] build release notes and save them in a docs/RELEASE_NOTES_0.1.0.md
- [ ] use `gh release create v0.1.0 --title "v0.1.0" --notes-file docs/RELEASE_NOTES_0.1_0.md`

### 3. Automatic Publishing
- GitHub Actions will automatically:
  - Build wheels for Linux, macOS (Intel & ARM), and Windows
  - Build source distribution
  - Publish to PyPI (if Trusted Publishing is configured)
  - Or fail and require manual intervention

## Post-Release

- [ ] Verify package on PyPI: https://pypi.org/project/loopsleuth/
- [ ] Test installation from PyPI: `pip install loopsleuth`
- [ ] Update any project documentation with release announcement
- [ ] Consider announcing on social media, Python forums, etc.

## Release Notes Template

```markdown
# LoopSleuth 0.1.0 - First Release

Initial release of LoopSleuth, an AI-powered Python performance analyzer that detects algorithmic bottlenecks using local LLM inference.

## Features

- 8 built-in performance checks (quadratic complexity, linear-in-loop, n+1, etc.)
- Local LLM inference using llama.cpp
- Intelligent SQLite-based caching
- Flexible TOML configuration
- Support for custom checks
- Interactive model download
- Multi-platform support (Linux, macOS, Windows)

## Installation

```bash
pip install loopsleuth
loopsleuth download-model
loopsleuth -m ~/.loopsleuth/models/qwen*.gguf ./src
```

See the [README](https://github.com/tarekziade/loopsleuth#readme) for full documentation.
```
