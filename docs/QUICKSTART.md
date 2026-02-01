# LoopSleuth Quick Start Guide

Get started with LoopSleuth in 3 simple steps!

## Step 1: Install LoopSleuth

### Option A: Install from PyPI (Recommended - Coming Soon)

```bash
pip install loopsleuth
```

### Option B: Install from GitHub

```bash
pip install git+https://github.com/yourusername/loopsleuth.git
```

### Option C: Install from Source

```bash
git clone https://github.com/yourusername/loopsleuth.git
cd loopsleuth
pip install .
```

**Note**: Installation requires Rust toolchain. Install from https://rustup.rs/ if needed.

## Step 2: Download a Model

After installation, download a model interactively:

```bash
loopsleuth download-model
```

This will:
1. Show you available models with size and description
2. Let you choose which one to download
3. Download it to `~/.loopsleuth/models/`
4. Show you how to use it

**Recommended model**: Qwen2.5-Coder (3B) - Best balance of speed and accuracy for code analysis.

### Alternative: Download with huggingface-cli

```bash
# Install hf CLI
pip install huggingface_hub

# Download recommended model
huggingface-cli download Qwen/Qwen2.5-Coder-3B-Instruct-GGUF \
    qwen2.5-coder-3b-instruct-q4_k_m.gguf \
    --local-dir ~/.loopsleuth/models
```

## Step 3: Analyze Your Code

Run LoopSleuth on your Python code:

```bash
# Analyze a single file
loopsleuth -m ~/.loopsleuth/models/qwen2.5-coder-3b-instruct-q4_k_m.gguf my_script.py

# Analyze a whole directory
loopsleuth -m ~/.loopsleuth/models/qwen2.5-coder-3b-instruct-q4_k_m.gguf ./src

# With short model path (if in default location)
loopsleuth -m ~/.loopsleuth/models/qwen*.gguf ./src
```

## Common Commands

```bash
# List downloaded models
loopsleuth list-models

# Show available performance checks
loopsleuth --list-checks

# Run specific checks only
loopsleuth -m <model> ./src --checks quadratic,linear-in-loop

# Get detailed report with solutions
loopsleuth -m <model> ./src --details

# Save report to file
loopsleuth -m <model> ./src --output report.md

# Show all options
loopsleuth --help
```

## Simplify Model Path with Environment Variable

Instead of typing the full model path every time:

```bash
# Add to ~/.bashrc or ~/.zshrc
export LOOPSLEUTH_MODEL="$HOME/.loopsleuth/models/qwen2.5-coder-3b-instruct-q4_k_m.gguf"

# Then use:
loopsleuth -m $LOOPSLEUTH_MODEL ./src
```

Or create an alias:

```bash
# Add to ~/.bashrc or ~/.zshrc
alias loopsleuth='loopsleuth -m ~/.loopsleuth/models/qwen2.5-coder-3b-instruct-q4_k_m.gguf'

# Then just use:
loopsleuth ./src
```

## Example Session

```bash
$ pip install loopsleuth
# ... installation output ...

$ loopsleuth download-model

╔═══════════════════════════════════════════════════════════╗
║           LoopSleuth Model Download                      ║
╚═══════════════════════════════════════════════════════════╝

Choose a model to download:

1. Qwen2.5-Coder (3B) - Recommended ⭐ (~2GB)
   Best for code analysis, excellent accuracy

2. Devstral Small 2 (24B) (~15GB)
   Highest accuracy, requires more RAM

3. Qwen2.5 (3B) (~2GB)
   General purpose, good balance

4. Qwen2.5 (0.5B) (~400MB)
   Very fast, lower accuracy

0. Exit without downloading

Enter choice (1-4, 0 to exit) [1]: 1

📥 Downloading Qwen2.5-Coder (3B) - Recommended ⭐...
   Repository: Qwen/Qwen2.5-Coder-3B-Instruct-GGUF
   File: qwen2.5-coder-3b-instruct-q4_k_m.gguf
   Size: ~2GB
   Destination: /Users/you/.loopsleuth/models

   This may take several minutes depending on your connection...

✅ Download complete!
   Model saved to: /Users/you/.loopsleuth/models/qwen2.5-coder-3b-instruct-q4_k_m.gguf

============================================================
  LoopSleuth is ready! 🎉
============================================================

Run analysis with:
  loopsleuth -m /Users/you/.loopsleuth/models/qwen2.5-coder-3b-instruct-q4_k_m.gguf <path_to_python_code>

Example:
  loopsleuth -m /Users/you/.loopsleuth/models/qwen2.5-coder-3b-instruct-q4_k_m.gguf ./my_project/

$ loopsleuth -m ~/.loopsleuth/models/qwen*.gguf ./my_project/

🔍 Analyzing Python code for performance issues...

╔═══════════════════════════════╗
║ LOOPSLEUTH ANALYSIS SUMMARY   ║
╚═══════════════════════════════╝

📊 Total functions analyzed: 15
🔍 Checks run: 8 (quadratic, linear-in-loop, n-plus-one, ...)
⚠️  Functions with issues: 3
✓  Functions clean: 12

🔴 ISSUES DETECTED:
─────────────────────────────────────────────────────────────
  • find_duplicates (utils.py:42)
    - Quadratic Complexity

  • load_config (config.py:10)
    - N+1 Problem

  • process_batch (processor.py:89)
    - Linear Operations in Loops

💡 Tip: Use --details to see full analysis or --output FILE to save report
```

## Integration with Development Workflow

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
        args: ["-m", "~/.loopsleuth/models/qwen2.5-coder-3b-instruct-q4_k_m.gguf", "--checks", "quadratic"]
        pass_filenames: true
```

### GitHub Actions

```yaml
- name: Install LoopSleuth
  run: |
    pip install loopsleuth
    loopsleuth download-model  # Interactive input needed, or use hf CLI

- name: Run Analysis
  run: loopsleuth -m ~/.loopsleuth/models/qwen*.gguf ./src --output report.md
```

### VS Code Task

Add to `.vscode/tasks.json`:

```json
{
  "version": "2.0.0",
  "tasks": [
    {
      "label": "LoopSleuth Analysis",
      "type": "shell",
      "command": "loopsleuth -m ~/.loopsleuth/models/qwen*.gguf ${file}",
      "problemMatcher": [],
      "presentation": {
        "reveal": "always",
        "panel": "new"
      }
    }
  ]
}
```

## Troubleshooting

### "huggingface_hub not found"
```bash
pip install huggingface_hub
```

### "Model file not found"
```bash
# List your models
loopsleuth list-models

# Download a new one
loopsleuth download-model
```

### "Binary not found"
The Rust binary might not have been built during installation. Try:
```bash
pip install --force-reinstall --no-cache-dir loopsleuth
```

### Slow Analysis
- Use a smaller model (Qwen2.5 0.5B) for faster results
- Use `--checks` to run only specific checks
- Enable caching (enabled by default) for repeat analysis

## Next Steps

- Read the [full documentation](docs/)
- Check out [available performance checks](README.md#performance-checks)
- Learn about [configuration options](docs/PYTHON_INSTALL.md)
- See [examples and use cases](README.md#usage)

## Getting Help

```bash
# Show all CLI options
loopsleuth --help

# List available checks
loopsleuth --list-checks

# See example output
loopsleuth --help | grep -A20 "EXAMPLES"
```

For more help, see the [full documentation](README.md) or [file an issue](https://github.com/yourusername/loopsleuth/issues).
