# Bootstrap Feature - Model Download Integration

## Overview

The bootstrap feature integrates model downloading directly into the Python package, providing a seamless installation experience. Users install LoopSleuth via pip and download models interactively through the CLI.

## Implementation

### Components

1. **`python/loopsleuth/models.py`** - Model management module
   - Model definitions with metadata (name, size, description)
   - Interactive download menu
   - Integration with `huggingface_hub` library
   - Model directory management (`~/.loopsleuth/models/`)
   - List downloaded models

2. **`python/loopsleuth/__main__.py`** - Enhanced CLI entry point
   - Subcommand routing (`download-model`, `list-models`)
   - Custom help that includes Python subcommands
   - Passthrough to Rust binary for analysis commands

3. **`pyproject.toml`** - Package configuration
   - Added `loopsleuth-download-model` entry point
   - Added optional dependency group for model downloading
   - `huggingface_hub` as optional dependency

### New Commands

```bash
# Download a model interactively
loopsleuth download-model
# or shorter:
loopsleuth download

# Also available as standalone command:
loopsleuth-download-model

# List downloaded models
loopsleuth list-models
```

### Model Storage

Models are stored in `~/.loopsleuth/models/` by default. This can be customized with:
```bash
export LOOPSLEUTH_MODELS_DIR=/path/to/models
```

## User Workflow

### Standard Installation (Current)

```bash
pip install loopsleuth
loopsleuth download-model  # Interactive model selection
loopsleuth -m ~/.loopsleuth/models/qwen*.gguf ./src
```

### Development/Source Build (Advanced)

```bash
git clone repo
cd repo
pip install -e .  # Installs in development mode
loopsleuth download-model
loopsleuth -m ~/.loopsleuth/models/qwen*.gguf ./src
# Or: ./target/release/loopsleuth -m models/model.gguf ./src
```

## Features

### Interactive Model Selection

When running `loopsleuth download-model`, users see:

```
╔═══════════════════════════════════════════════════════════╗
║           LoopSleuth Model Download                      ║
╚═══════════════════════════════════════════════════════════╝

Choose a model to download:

1. Qwen2.5-Coder (7B) - Recommended ⭐ (~4.7GB)
   Best for code analysis, excellent accuracy

2. Qwen2.5-Coder (3B) (~2GB)
   Faster but less accurate on harder ML-specific checks

3. Devstral Small 2 (24B) (~15GB)
   Highest accuracy, requires more RAM

4. Qwen2.5 (3B) (~2GB)
   General purpose, good balance

5. Qwen2.5 (0.5B) (~400MB)
   Very fast, lower accuracy

6. Qwen3.5 (2B) (~1.3GB)
   Compact newer general-purpose alternative

7. Qwen3.5 (4B) (~3GB)
   Stronger compact alternative with better accuracy than 2B

8. Gemma 4 (E2B) (~3.1GB)
   Good alternative for local reasoning and code analysis

9. Custom model (provide Hugging Face URL) (varies)
   Download from a custom Hugging Face repository

0. Exit without downloading

Enter choice (1-9, 0 to exit) [1]:
```

### Smart Dependency Management

The module checks for `huggingface_hub` and offers to install it if missing:

```python
try:
    import huggingface_hub
except ImportError:
    print("⚠️  huggingface_hub is not installed")
    print("\n   Installing huggingface_hub...")
    subprocess.check_call([sys.executable, "-m", "pip", "install", "-q", "huggingface_hub"])
```

### Model Information

Each model includes:
- **Name**: Display name with size
- **Repository**: Hugging Face repo ID
- **Filename**: Specific GGUF file
- **Size**: Approximate download size
- **Description**: Use case and characteristics

### Download Progress

Uses `huggingface_hub.hf_hub_download()` which shows:
- Progress bar during download
- Download speed
- ETA

### Post-Download Instructions

After successful download:
```
✅ Download complete!
   Model saved to: /Users/you/.loopsleuth/models/Qwen2.5-Coder-7B-Instruct-128K-Q4_K_M.gguf

============================================================
  LoopSleuth is ready! 🎉
============================================================

Run analysis with:
  loopsleuth -m /Users/you/.loopsleuth/models/Qwen2.5-Coder-7B-Instruct-128K-Q4_K_M.gguf <path_to_python_code>

Example:
  loopsleuth -m /Users/you/.loopsleuth/models/Qwen2.5-Coder-7B-Instruct-128K-Q4_K_M.gguf ./my_project/
```

## Technical Details

### Model Definitions

```python
MODELS = {
    "1": {
        "name": "Qwen2.5-Coder (7B) - Recommended ⭐",
        "repo": "unsloth/Qwen2.5-Coder-7B-Instruct-128K-GGUF",
        "filename": "Qwen2.5-Coder-7B-Instruct-128K-Q4_K_M.gguf",
        "size": "~4.7GB",
        "description": "Best for code analysis, excellent accuracy",
    },
    # ... more models, including Qwen3.5 (2B), Qwen3.5 (4B), Gemma 4 (E2B), and custom repo support
}
```

### Directory Management

```python
def get_models_dir() -> Path:
    """Get the default models directory.

    Priority:
    1. LOOPSLEUTH_MODELS_DIR environment variable
    2. ~/.loopsleuth/models/
    """
    if "LOOPSLEUTH_MODELS_DIR" in os.environ:
        return Path(os.environ["LOOPSLEUTH_MODELS_DIR"])
    return Path.home() / ".loopsleuth" / "models"
```

### Download Implementation

```python
def download_model(choice: str, models_dir: Path) -> Optional[Path]:
    """Download the selected model using huggingface_hub."""
    model = MODELS[choice]

    downloaded_path = hf_hub_download(
        repo_id=model['repo'],
        filename=model['filename'],
        local_dir=models_dir,
        local_dir_use_symlinks=False,  # Copy instead of symlink
    )

    return Path(downloaded_path)
```

### CLI Subcommand Routing

```python
def main():
    """Run the LoopSleuth binary with provided arguments."""
    args = sys.argv[1:]

    # Handle download-model subcommand
    if args and args[0] in ("download-model", "download"):
        from .models import main as download_main
        sys.exit(download_main())

    # Handle list-models subcommand
    if args and args[0] == "list-models":
        from .models import list_downloaded_models, get_models_dir
        # ... show models

    # Otherwise, run the Rust binary
    binary_path = get_binary_path()
    subprocess.run([binary_path] + args)
```

## Benefits

### For Users

1. **Single install command**: `pip install loopsleuth`
2. **Guided model selection**: No need to know Hugging Face repo details
3. **Automatic setup**: Models go to standard location
4. **No manual downloads**: Everything through the CLI
5. **Persistent storage**: Models survive across upgrades

### For Developers

1. **No bash scripts**: Pure Python implementation
2. **Cross-platform**: Works on Windows, macOS, Linux
3. **Extensible**: Easy to add new models
4. **Testable**: Python code is easier to test than bash
5. **Better error handling**: Python exceptions vs bash exit codes

### For CI/CD

```yaml
# Old way (setup.sh)
- run: |
    ./setup.sh
    # Requires manual model selection or env vars
    ./target/release/loopsleuth -m models/model.gguf ./src

# New way (pip + bootstrap)
- run: |
    pip install loopsleuth
    # Can script model download or use pre-downloaded
    loopsleuth -m ~/.loopsleuth/models/qwen*.gguf ./src
```

## Backwards Compatibility

- Source builds still supported via `pip install -e .`
- Both model locations work:
  - Standard: `~/.loopsleuth/models/` (recommended)
  - Legacy: `models/` in project directory (for source builds)
- No breaking changes to existing functionality

## Testing

### Manual Testing

```bash
# Install in development mode
pip install -e .

# Test download command
loopsleuth download-model

# Test list command
loopsleuth list-models

# Test that binary still works
loopsleuth --help
```

### Automated Testing

```bash
# Run bootstrap test script
./test_bootstrap.sh
```

This creates a virtual environment, installs LoopSleuth, and verifies all commands work.

## Documentation Updates

1. **README.md**: Updated installation section to highlight pip + bootstrap workflow
2. **docs/QUICKSTART.md**: New quick start guide featuring the 3-command setup
3. **docs/PYTHON_INSTALL.md**: Updated with bootstrap instructions
4. **AGENTS.md**: Updated with new commands and project structure
5. **pyproject.toml**: Added optional dependencies for model downloading

## Future Enhancements

1. **Model caching**: Check if model already downloaded before re-downloading
2. **Model updates**: Detect and offer to update outdated models
3. **Custom models**: Allow users to add custom model definitions
4. **Model validation**: Verify downloaded models are valid GGUF files
5. **Parallel downloads**: Download multiple models at once
6. **Model search**: Search Hugging Face for compatible models
7. **Non-interactive mode**: `--model-choice 1` for automation
8. **Configuration storage**: Remember user's preferred model

## Migration from Source Builds

For users who were building from source:

```bash
# Old workflow (source build)
git clone repo && cd repo
cargo build --release
./target/release/loopsleuth -m models/qwen*.gguf ./src

# New workflow (pip install)
pip install loopsleuth
loopsleuth download-model  # Downloads to ~/.loopsleuth/models/
loopsleuth -m ~/.loopsleuth/models/qwen*.gguf ./src

# Optional: Move existing models to standard location
mkdir -p ~/.loopsleuth/models
mv models/*.gguf ~/.loopsleuth/models/
```

## Files Modified/Created

### Created
- `python/loopsleuth/models.py` - Model management module
- `docs/QUICKSTART.md` - Quick start guide
- `docs/BOOTSTRAP.md` - This file
- `test_bootstrap.sh` - Bootstrap testing script

### Modified
- `python/loopsleuth/__main__.py` - Added subcommand routing
- `python/loopsleuth/__init__.py` - Exported model functions
- `pyproject.toml` - Added entry points and optional dependencies
- `README.md` - Updated installation section
- `AGENTS.md` - Updated documentation index and project structure

## Summary

The bootstrap feature makes LoopSleuth a fully pip-installable package with guided setup, providing a standard Python package experience. Users can go from zero to analyzing code in three simple commands:

```bash
pip install loopsleuth
loopsleuth download-model
loopsleuth -m ~/.loopsleuth/models/qwen*.gguf ./src
```

This approach dramatically improves the user experience and makes LoopSleuth accessible to all Python developers who expect standard `pip install` workflows.
