# LoopSleuth 0.1.0 - First Release

Initial release of LoopSleuth, an AI-powered Python performance analyzer that detects algorithmic bottlenecks using local LLM inference.

## Features

- 8 built-in performance checks (quadratic complexity, linear-in-loop, n+1, etc.)
- Local LLM inference using llama.cpp
- Intelligent SQLite-based caching for improved performance
- Flexible TOML configuration
- Support for custom checks
- Interactive model download via `loopsleuth download-model`
- Multi-platform support (Linux, macOS Intel & ARM, Windows)
- Real-time progress bar with feedback
- Abort handling for graceful interruption

## Installation

```bash
pip install loopsleuth
loopsleuth download-model
loopsleuth -m ~/.loopsleuth/models/qwen*.gguf ./src
```

## Technical Highlights

- Rust-based llama.cpp bindings for optimal performance
- Python API for easy integration
- Cross-platform wheel builds via GitHub Actions
- Efficient caching system to avoid redundant analysis

## What's Included

This release includes:
- Core detection engine
- CLI interface
- Model management utilities
- Comprehensive documentation
- Example configurations

See the [README](https://github.com/tarekziade/loopsleuth#readme) for full documentation and usage examples.

## Contributors

- Tarek Ziadé (@tarekziade)
