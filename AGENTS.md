# LoopSleuth - AI Agent Documentation

## Project Overview

**LoopSleuth** is a Rust-based CLI tool that analyzes Python code for quadratic O(n²) complexity patterns using local LLM inference via llama.cpp.

**Purpose**: Help developers identify and fix performance bottlenecks in Python codebases by combining static analysis with AI-powered code understanding.

**Status**: Production-ready, fully functional

## Documentation Index

When working on this project, refer to these documents for detailed information:

### Core Documentation
- **[README.md](README.md)** - User documentation, installation guide, and usage examples
- **[AGENTS.md](AGENTS.md)** - This file - Quick reference for AI agents working on the codebase

### Technical Documentation (in `docs/`)
- **[docs/QUICKSTART.md](docs/QUICKSTART.md)** - Quick start guide for new users (pip install + download-model)
- **[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)** - Technical design decisions, implementation details, and system architecture
- **[docs/COMPARISON.md](docs/COMPARISON.md)** - Comparison with alternative tools and approaches
- **[docs/CACHE_IMPLEMENTATION.md](docs/CACHE_IMPLEMENTATION.md)** - SQLite cache system implementation, performance impact, and usage
- **[docs/PYTHON_INSTALL.md](docs/PYTHON_INSTALL.md)** - Comprehensive pip installation guide with CI/CD integration
- **[docs/PIP_INSTALL_SETUP.md](docs/PIP_INSTALL_SETUP.md)** - Implementation details of the pip package setup
- **[docs/PYPI_PUBLISHING.md](docs/PYPI_PUBLISHING.md)** - Guide for publishing to PyPI with GitHub Actions

### Documentation Conventions

**IMPORTANT**: All technical documentation should be created and maintained in the `docs/` directory.

- **User-facing files in root**: README.md, LICENSE, AGENTS.md only
- **Technical documentation**: Always in `docs/` directory
- **When creating new documentation**: Place in `docs/` and update this index
- **When updating features**: Update relevant docs in `docs/` and README.md

## Quick Start for Agents

### For Development (Building from Source)

```bash
# Build
cargo build --release

# Run
./target/release/loopsleuth --model <model.gguf> <python_file_or_directory>

# Test
cargo run --release -- --model ./models/qwen2.5-coder-3b-instruct-q4_k_m.gguf ./test_examples
```

### For Testing Pip Package

```bash
# Install in development mode
pip install -e .

# Download a model
loopsleuth download-model

# List models
loopsleuth list-models

# Run analysis
loopsleuth -m ~/.loopsleuth/models/qwen*.gguf ./test_examples

# With details
loopsleuth -m ~/.loopsleuth/models/qwen*.gguf ./test_examples --details
```

## Architecture

### Technology Stack
- **Language**: Rust 2021 edition
- **Parser**: RustPython (rustpython-parser, rustpython-ast)
- **LLM**: llama.cpp via llama-cpp-2 bindings
- **CLI**: clap 4.5
- **File traversal**: walkdir
- **Cache**: rusqlite (SQLite with bundled feature)
- **Hashing**: sha2 (SHA256 for function fingerprints)

### Key Components

1. **CLI Layer** (`main.rs`)
   - Argument parsing with clap
   - Progress feedback
   - Output formatting

2. **Python Analysis**
   - Parse Python files to AST
   - Extract function definitions recursively
   - Support for both files and directories

3. **Cache System** (see [docs/CACHE_IMPLEMENTATION.md](docs/CACHE_IMPLEMENTATION.md))
   - SQLite database for persistent storage
   - SHA256 hashing for function fingerprinting
   - Automatic cache invalidation on code changes
   - ~100x speedup on repeated runs

4. **LLM Integration**
   - Two-stage analysis: detection → solution
   - Greedy sampling for deterministic results
   - Stderr suppression for clean output

5. **Output Generation**
   - Concise summary (default)
   - Detailed markdown report (--details)
   - File export (--output)

## Project Structure

```
LoopSleuth/
├── src/
│   └── main.rs              # ~1200 lines, all Rust logic (includes cache)
├── python/                  # Python package
│   └── loopsleuth/
│       ├── __init__.py      # Package exports
│       ├── __main__.py      # CLI entry point with subcommands
│       └── models.py        # Model download/management
├── docs/                    # Documentation
│   ├── ARCHITECTURE.md      # Technical design
│   ├── COMPARISON.md        # Tool comparisons
│   ├── CACHE_IMPLEMENTATION.md  # Cache system details
│   ├── PYTHON_INSTALL.md    # Pip installation guide
│   ├── PIP_INSTALL_SETUP.md # Implementation details
│   ├── PYPI_PUBLISHING.md   # PyPI publishing guide
│   └── QUICKSTART.md        # Quick start guide
├── .github/
│   └── workflows/
│       ├── publish.yml      # PyPI publishing workflow
│       └── test-build.yml   # CI testing workflow
├── test_examples/           # Python test files
│   ├── sample.py
│   └── performance_issues.py
├── examples/
│   └── test_parse.rs        # Parser testing
├── Cargo.toml               # Rust dependencies & binary config
├── pyproject.toml           # Python package metadata
├── setup.py                 # setuptools-rust configuration
├── MANIFEST.in              # Package file inclusion rules
├── loopsleuth.toml          # Default configuration
├── README.md                # User documentation
├── AGENTS.md                # This file (agent quick reference)
├── LICENSE                  # MIT license
├── setup.sh                 # Interactive setup (legacy)
├── Makefile                 # Build commands
└── .gitignore

Not in git:
├── .loopsleuth_cache/       # SQLite cache database
├── ~/.loopsleuth/models/    # GGUF model files (~2-15GB) [pip install]
├── models/                  # GGUF model files [source build]
├── target/                  # Rust build artifacts
├── build/                   # Python build artifacts
├── dist/                    # Python distribution packages
└── report.md                # Generated reports
```

## Development Workflow

### Making Changes

1. **Code is in single file**: `src/main.rs`
2. **No tests yet**: Use `test_examples/` for manual testing
3. **Build**: `cargo build --release` (takes ~10s)
4. **Test**: Run against test files
5. **Check warnings**: `cargo clippy`
6. **Verify YAML**: Use `yamllint` for GitHub workflows and config files

### Common Tasks

**Add new CLI flag:**
- Update `Cli` struct with `#[derive(Parser)]`
- Add field with `#[arg(...)]` attribute

**Modify output format:**
- Update `print_summary()` or `print_detailed_report()`
- Check both single-file and multi-file cases

**Change LLM prompts:**
- Update `analyze_complexity()` for detection
- Update `propose_solution()` for fixes

**Add new complexity pattern:**
- Modify system prompt in `analyze_complexity()`
- Add test case to `test_examples/`

**Modify cache behavior:**
- See `AnalysisCache` struct in `main.rs`
- Database schema in `AnalysisCache::new()`
- Cache key generation in `AnalysisCache::hash_function()`
- Full details in [docs/CACHE_IMPLEMENTATION.md](docs/CACHE_IMPLEMENTATION.md)

**Validate YAML files:**
```bash
# Check GitHub workflow syntax
yamllint .github/workflows/*.yml

# Use relaxed rules (fewer style warnings)
yamllint -d relaxed .github/workflows/*.yml

# Check specific file
yamllint .github/workflows/test-build.yml
```

## Important Implementation Details

### Stderr Suppression
```rust
struct StderrSuppressor {
    // RAII guard that redirects stderr to /dev/null
    // Prevents llama.cpp debug logs from polluting output
}
```
- Uses `libc::dup2()` to redirect file descriptors
- Restored automatically on drop
- Can be bypassed with `--verbose` flag

### Progress Feedback
- Initialization messages shown before suppressor
- Dots (`.`) for per-function progress
- Uses `stdout` to avoid suppression

### AST Traversal
```rust
// Recursive function extraction
fn extract_functions_from_body(body: &[Stmt], ...) {
    match stmt {
        Stmt::FunctionDef(func) => { /* extract */ }
        Stmt::ClassDef(class) => { /* recurse */ }
        // ...
    }
}
```

### Result Grouping
- Results collected per-file into `FileResults`
- Flattened for compatibility with existing code
- Grouped display for multi-file analysis

## Testing Strategy

### Manual Testing
```bash
# Single file
./target/release/loopsleuth -m model.gguf test_examples/sample.py

# Directory
./target/release/loopsleuth -m model.gguf test_examples/

# With details
./target/release/loopsleuth -m model.gguf test_examples/ --details

# Save report
./target/release/loopsleuth -m model.gguf test_examples/ -o report.md

# Verbose (show llama.cpp logs)
./target/release/loopsleuth -m model.gguf test_examples/ --verbose
```

### What to Test
- ✅ Single file analysis
- ✅ Directory recursion
- ✅ Output modes (default, --details, --output)
- ✅ Progress indicators
- ✅ Error handling (bad paths, parse errors)
- ✅ Large directories (performance)

## Common Issues & Solutions

### Build Failures
- **cmake not found**: Install with `brew install cmake` (macOS) or `apt install cmake` (Linux)
- **llama-cpp-2 fails**: Ensure cmake and C++ compiler available

### Runtime Issues
- **Model loading slow**: Normal, 3-10 seconds for 3B models
- **No output**: Check stderr wasn't redirected (use --verbose)
- **Parse errors**: RustPython doesn't support all Python syntax extensions

### Output Issues
- **Logs still showing**: StderrSuppressor only works after creation
- **Dots not appearing**: Need to flush stdout after each print
- **Formatting broken**: Check Unicode box drawing characters

## Recommended Models

| Model | Size | Speed | Accuracy | Best For |
|-------|------|-------|----------|----------|
| **Qwen2.5-Coder-3B** | 2GB | Fast | Excellent | Recommended |
| Devstral Small 2 (24B) | 15GB | Slow | Excellent | Deep analysis |
| Qwen2.5-0.5B | 400MB | Very Fast | Fair | Quick checks |

## Code Patterns

### Adding New Flag
```rust
// In Cli struct
#[arg(short, long, help = "Description")]
my_flag: bool,

// In main()
if cli.my_flag {
    // Handle flag
}
```

### Modifying Output
```rust
// Concise: edit print_summary()
// Detailed: edit print_detailed_report()
// File: edit write_report_to_file()
```

### Changing Prompts
```rust
// Detection
fn analyze_complexity(...) {
    let prompt = format!(r#"
        <|im_start|>system
        Your detection prompt here
        <|im_end|>
        ..."#);
}

// Solution
fn propose_solution(...) {
    let prompt = format!(r#"
        <|im_start|>system
        Your solution prompt here
        <|im_end|>
        ..."#);
}
```

## Future Enhancements

### Priority
1. **Unit tests**: Add proper test suite
2. **Config file**: `.loopsleuth.toml` for settings
3. **JSON output**: For CI/CD integration
4. ~~**Caching**: Remember analyzed functions~~ ✅ **IMPLEMENTED** (see docs/CACHE_IMPLEMENTATION.md)

### Nice to Have
- Cache enhancements (TTL, size limits, sharing)
- Multiple language support
- Batch processing (parallel analysis)
- VS Code extension
- Pre-commit hook
- Web UI for results

## Performance Characteristics

- **Model loading**: 3-10s (one-time)
- **Per function**: 2-8s (2 LLM calls if quadratic)
- **Memory**: ~500MB + model size
- **Binary size**: 6.6MB
- **Typical analysis**: 10 functions = 1-2 minutes

## Contributing Guidelines

When modifying this project:

1. **Keep it simple**: Single file architecture is intentional
2. **Test manually**: Use test_examples/ before committing
3. **Update docs**: Keep README.md and ARCHITECTURE.md in sync
4. **Clean output**: Ensure terminal output stays clean
5. **Progress feedback**: Always show what's happening
6. **Handle errors**: Don't crash, report and continue

## Resources

### External Documentation
- **Rust docs**: https://doc.rust-lang.org/
- **RustPython**: https://github.com/RustPython/RustPython
- **llama.cpp**: https://github.com/ggerganov/llama.cpp
- **clap**: https://docs.rs/clap/
- **rusqlite**: https://docs.rs/rusqlite/

### Internal Documentation
- **[README.md](README.md)** - User guide and installation
- **[docs/ARCHITECTURE.md](docs/ARCHITECTURE.md)** - System design and technical details
- **[docs/COMPARISON.md](docs/COMPARISON.md)** - Alternative tools and approaches
- **[docs/CACHE_IMPLEMENTATION.md](docs/CACHE_IMPLEMENTATION.md)** - Cache system design and usage

## Contact & Support

This is an educational project demonstrating:
- Rust + Python AST analysis
- Local LLM integration
- SQLite-based caching for performance
- Clean CLI UX design
- Practical code optimization

For questions or issues, refer to the documentation files listed above.

---

*This document is designed to help AI agents/LLMs understand and work with the LoopSleuth codebase effectively.*
