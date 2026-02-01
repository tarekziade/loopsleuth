# LoopSleuth 0.1.1 - Quality and Platform Improvements

This release focuses on improving detection accuracy and expanding platform support.

## What's New

### Model Improvements
- **Switched to 7B model**: Upgraded from Qwen2.5-Coder-3B to 7B variant to significantly reduce hallucination issues and improve detection accuracy
- **Enhanced N+1 detection prompts**: Improved prompt engineering to better distinguish between necessary loop operations and true N+1 patterns

### Platform Support
- **Expanded Python version support**: Now supports Python 3.10 through 3.14
- **Improved Linux builds**: Enhanced manylinux compatibility and fixed clang compilation issues
- **Fixed Windows builds**: Resolved Rust installation issues in Windows CI pipeline
- **Streamlined macOS builds**: Removed deprecated macOS-13 target

### Testing & Quality
- Added comprehensive test examples for N+1 pattern detection
- Enhanced CI/CD workflows for more reliable builds across platforms

## Upgrade Notes

### Model Download
If you're upgrading from 0.1.0, you'll want to download the new 7B model for improved accuracy:

```bash
pip install --upgrade loopsleuth
loopsleuth download-model
```

The new model will be downloaded to `~/.loopsleuth/models/`. You can specify it with:

```bash
loopsleuth -m ~/.loopsleuth/models/qwen2.5-coder-7b-instruct-q4_k_m.gguf ./src
```

### Configuration
The configuration format remains compatible with 0.1.0. No changes needed to existing `loopsleuth.toml` files.

## Bug Fixes

- Fixed hallucination issues in detection by upgrading to larger model
- Resolved build issues on Linux with clang
- Fixed Windows Rust toolchain installation in CI

## Technical Details

- Model: Now defaults to Qwen2.5-Coder-7B-Instruct (Q4_K_M quantization)
- Build: Enhanced cross-platform wheel builds (manylinux, macOS, Windows)
- Testing: Added 269 lines of N+1 test examples

## Contributors

- Tarek Ziadé (@tarekziade)

---

For full documentation, see the [README](https://github.com/tarekziade/loopsleuth#readme).
