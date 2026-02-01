# Installing LoopSleuth via pip

LoopSleuth can be installed as a Python package using pip, making it easy to integrate into Python projects.

## Prerequisites

### System Requirements
- Python 3.8 or later
- Rust toolchain (for building from source)
  - Install from https://rustup.rs/
  - Or use pre-built wheels if available

### Install Rust (if needed)
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
```

## Installation Methods

### Method 1: Install from Source (Recommended)

```bash
# Install directly from the repository
pip install git+https://github.com/yourusername/loopsleuth.git

# Or clone and install locally
git clone https://github.com/yourusername/loopsleuth.git
cd loopsleuth
pip install .
```

### Method 2: Install in Development Mode

For development or testing:

```bash
git clone https://github.com/yourusername/loopsleuth.git
cd loopsleuth
pip install -e .
```

This creates an editable installation where changes to the Python code take effect immediately.

### Method 3: Install from PyPI (Future)

Once published to PyPI:

```bash
pip install loopsleuth
```

## Usage After Installation

Once installed, `loopsleuth` is available as a command-line tool:

```bash
# Download a model first (one-time setup)
mkdir -p models
wget https://huggingface.co/Qwen/Qwen2.5-Coder-3B-Instruct-GGUF/resolve/main/qwen2.5-coder-3b-instruct-q4_k_m.gguf -P models/

# Run analysis
loopsleuth -m models/qwen2.5-coder-3b-instruct-q4_k_m.gguf ./my_python_code/

# Or use as a Python module
python -m loopsleuth -m models/qwen2.5-coder-3b-instruct-q4_k_m.gguf ./my_python_code/
```

## Integration in Python Projects

Add to your project's development dependencies:

### Using requirements.txt
```txt
# requirements-dev.txt
loopsleuth @ git+https://github.com/yourusername/loopsleuth.git
```

### Using pyproject.toml (Poetry/modern setuptools)
```toml
[project.optional-dependencies]
dev = [
    "loopsleuth @ git+https://github.com/yourusername/loopsleuth.git",
]
```

### Using Pipenv
```bash
pipenv install --dev loopsleuth
```

## Configuration

LoopSleuth looks for configuration in the following order:
1. `--config <path>` flag
2. `~/.config/loopsleuth/loopsleuth.toml`
3. Built-in defaults

You can generate a default config file:
```bash
loopsleuth --print-default-config > ~/.config/loopsleuth/loopsleuth.toml
```

## CI/CD Integration

### GitHub Actions Example

```yaml
name: Performance Analysis

on: [pull_request]

jobs:
  loopsleuth:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3

      - name: Set up Python
        uses: actions/setup-python@v4
        with:
          python-version: '3.11'

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install LoopSleuth
        run: pip install git+https://github.com/yourusername/loopsleuth.git

      - name: Download Model
        run: |
          mkdir -p models
          wget https://huggingface.co/Qwen/Qwen2.5-Coder-3B-Instruct-GGUF/resolve/main/qwen2.5-coder-3b-instruct-q4_k_m.gguf -P models/

      - name: Run Analysis
        run: |
          loopsleuth -m models/qwen2.5-coder-3b-instruct-q4_k_m.gguf ./src --output report.md

      - name: Upload Report
        uses: actions/upload-artifact@v3
        with:
          name: performance-report
          path: report.md
```

### Pre-commit Hook

Add to `.pre-commit-config.yaml`:

```yaml
repos:
  - repo: local
    hooks:
      - id: loopsleuth
        name: LoopSleuth Performance Check
        entry: loopsleuth
        language: system
        types: [python]
        args: ["-m", "models/qwen2.5-coder-3b-instruct-q4_k_m.gguf", "--checks", "quadratic"]
        pass_filenames: true
```

## Troubleshooting

### Build Fails: "Rust toolchain not found"
Install Rust: https://rustup.rs/

### Binary Not Found After Install
The binary may not have been built. Try:
```bash
pip install --force-reinstall --no-cache-dir loopsleuth
```

### Permission Denied When Running
Ensure the binary is executable:
```bash
chmod +x $(python -c "from loopsleuth import get_binary_path; print(get_binary_path())")
```

### Model Download Issues
Models are large (2-4GB). Use a stable connection and ensure you have enough disk space.

## Uninstalling

```bash
pip uninstall loopsleuth
```

## Building Wheels for Distribution

To build distributable wheels:

```bash
# Install build tools
pip install build setuptools-rust

# Build wheel
python -m build

# Wheels will be in dist/
ls -lh dist/
```

## Platform Support

- **Linux**: x86_64, aarch64
- **macOS**: x86_64, arm64 (Apple Silicon)
- **Windows**: x86_64

Pre-built wheels may not be available for all platforms. Building from source requires Rust toolchain.
