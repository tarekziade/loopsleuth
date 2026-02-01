# LoopSleuth

A Rust-based CLI tool that analyzes Python code for performance issues using local LLM inference.

## Installation

Get started in 3 commands:

```bash
# 1. Install LoopSleuth
pip install loopsleuth

# 2. Download a model interactively
loopsleuth download-model

# 3. Run analysis!
loopsleuth -m ~/.loopsleuth/models/qwen*.gguf ./src
```

That's it! The `download-model` command will show you available models, download your choice to `~/.loopsleuth/models/`, and show you how to use it.

**Quick Start Guide**: See [docs/QUICKSTART.md](docs/QUICKSTART.md) for a complete walkthrough.

## Features

- **Fully Configurable**: Define checks, customize prompts, and set defaults via TOML configuration file
- **8 Built-in Performance Checks**: Detects multiple types of performance issues beyond just quadratic complexity
- Parses Python code using Ruff's parser (fast and accurate)
- Extracts functions from Python modules
- Analyzes each function using a local LLM (llama.cpp)
- Supports both single files and entire directories
- **Intelligent caching** - Uses SQLite to cache analysis results per check, avoiding redundant LLM calls for unchanged functions
- **Flexible check selection** - Run all checks, specific checks, or exclude certain checks

## Performance Checks

LoopSleuth includes 8 built-in performance checks:

### General Performance
1. **quadratic** - Detects O(n²) or worse time complexity (nested loops, etc.)
2. **linear-in-loop** - Detects hidden O(n) operations in loops (`x in list`, `.remove()`, `.index()`)
3. **n-plus-one** - Detects repeated expensive operations in loops (file I/O, network, model loading)
4. **expensive-sort-key** - Detects O(n) key functions in sort/sorted operations
5. **unbounded-alloc** - Detects growing allocations in loops (string concat, repeated cat)
6. **growing-container** - Detects loops that grow containers while iterating

### ML-Specific
7. **conversion-churn** - Detects repeated CPU/GPU or tensor/array conversions in loops
8. **ml-footguns** - Detects ML-specific issues (repeated tokenization, mask rebuilding)

## Configuration

LoopSleuth uses a TOML configuration file (`loopsleuth.toml`) to define performance checks. You can:
- Customize existing checks
- Add your own custom checks
- Modify LLM prompts for better detection
- Set default CLI options

### Configuration File Locations

LoopSleuth looks for configuration in this order:
1. Path specified with `--config` flag
2. `~/.config/loopsleuth/loopsleuth.toml` (user config)
3. Built-in defaults (bundled with the tool)

### Configuration Format

```toml
[settings]
# Optional: Set default CLI options (can be overridden by command-line flags)
model = "./models/qwen2.5-coder-3b-instruct-q4_k_m.gguf"
threads = 4
max_tokens = 512
context_size = 4096

[[check]]
key = "my-custom-check"
name = "My Custom Check"
description = "Detects my specific performance pattern"
category = "performance"
keyword = "MY_ISSUE"  # Keyword LLM should include if issue detected
detection_prompt = """<|im_start|>system
You are a code analyzer...
Use {function_source} placeholder for the function code.
<|im_end|>
<|im_start|>user
Analyze: {function_source}
<|im_end|>
<|im_start|>assistant
"""
solution_prompt = """<|im_start|>system
Provide solutions...
<|im_end|>
<|im_start|>user
Fix this: {function_source}
<|im_end|>
<|im_start|>assistant
"""
```

### Using Custom Configuration

```bash
# Print default config to create your own
loopsleuth --print-default-config > my-loopsleuth.toml

# Edit my-loopsleuth.toml to customize checks or add new ones

# Use your custom config
loopsleuth --config my-loopsleuth.toml -m ~/.loopsleuth/models/qwen*.gguf ./src

# Or place it in ~/.config/loopsleuth/loopsleuth.toml for automatic loading
mkdir -p ~/.config/loopsleuth
cp my-loopsleuth.toml ~/.config/loopsleuth/loopsleuth.toml
```

### Adding Custom Checks

1. Get the default configuration:
   ```bash
   loopsleuth --print-default-config > ~/.config/loopsleuth/loopsleuth.toml
   ```

2. Add a new check section:
   ```toml
   [[check]]
   key = "database-in-loop"
   name = "Database Queries in Loop"
   description = "Detects database queries inside loops"
   category = "performance"
   keyword = "DB_IN_LOOP"
   detection_prompt = """..."""
   solution_prompt = """..."""
   ```

3. Run with your custom check:
   ```bash
   loopsleuth -m ~/.loopsleuth/models/qwen*.gguf ./src --checks database-in-loop
   ```

## Model Management

After installation, use these commands to manage models:

```bash
# Download a model interactively
loopsleuth download-model

# List downloaded models
loopsleuth list-models

# Use short form
loopsleuth download
```

**Recommended models**:
- **Qwen2.5-Coder (3B)** ⭐ - Best for code analysis (~2GB)
- **Devstral Small 2 (24B)** - Highest accuracy, requires more RAM (~15GB)
- **Qwen2.5 (3B)** - General purpose, good balance (~2GB)
- **Qwen2.5 (0.5B)** - Very fast, lower accuracy (~400MB)

The interactive download command will guide you through selecting and downloading the best model for your needs.

## Building from Source

For development or if you prefer to build from source:

**Prerequisites:**
- Rust toolchain from [rustup.rs](https://rustup.rs/)
- CMake (`brew install cmake` on macOS, `apt-get install cmake` on Linux)

```bash
# Clone the repository
git clone https://github.com/yourusername/loopsleuth.git
cd loopsleuth

# Build the project
cargo build --release

# Download a model
mkdir -p models
pip install huggingface_hub
hf download Qwen/Qwen2.5-Coder-3B-Instruct-GGUF \
  qwen2.5-coder-3b-instruct-q4_k_m.gguf \
  --local-dir ./models

# Run
./target/release/loopsleuth -m ./models/qwen*.gguf ./src
```

**Note**: The first build takes several minutes as it compiles llama.cpp from source. Subsequent builds are much faster.

For detailed build instructions and troubleshooting, see [docs/PYTHON_INSTALL.md](docs/PYTHON_INSTALL.md)

## Usage

### Basic Usage

Analyze a single Python file (runs all checks by default):
```bash
loopsleuth -m ~/.loopsleuth/models/qwen2.5-coder-3b-instruct-q4_k_m.gguf example.py
```

Analyze an entire directory (recursive):
```bash
loopsleuth -m ~/.loopsleuth/models/qwen*.gguf ./src
```

The tool automatically finds all `.py` files in subdirectories and groups results by file.

### Check Selection

List all available checks:
```bash
loopsleuth --list-checks
```

Run specific checks only:
```bash
loopsleuth -m ~/.loopsleuth/models/qwen*.gguf ./src --checks quadratic,linear-in-loop
```

Run all checks except specific ones:
```bash
loopsleuth -m ~/.loopsleuth/models/qwen*.gguf ./src --exclude conversion-churn,ml-footguns
```

**Note**: By default, all 8 checks are run. Use `--checks` to select specific checks or `--exclude` to skip certain checks.

### Options

#### Required
- `-m, --model <MODEL>` - Path to the GGUF model file (required unless using --list-checks)
- `<PATH>` - Path to Python file or directory to analyze (required unless using --list-checks)

#### Check Selection
- `--list-checks` - List all available checks and exit
- `--checks <CHECKS>` - Comma-separated list of checks to run (e.g., "quadratic,linear-in-loop")
- `--exclude <CHECKS>` - Comma-separated list of checks to exclude from analysis

#### Configuration
- `--config <FILE>` - Path to custom checks configuration file (TOML format)
- `--print-default-config` - Print the built-in default configuration and exit

#### LLM Options
- `-t, --threads <THREADS>` - Number of threads for inference (default: 4)
- `--max-tokens <MAX_TOKENS>` - Maximum tokens to generate (default: 512)
- `--context-size <SIZE>` - Context window size in tokens (default: 4096)
- `-v, --verbose` - Show verbose llama.cpp output (useful for debugging)

#### Output Options
- `-o, --output <FILE>` - Save analysis report to markdown file
- `-d, --details` - Show detailed report in stdout (always included in file output)
- `--skip-large <N>` - Skip functions larger than N lines (0 = no limit)

#### Cache Options
- `--no-cache` - Disable caching (forces re-analysis of all functions)
- `--clear-cache` - Clear the cache before running analysis
- `--cache-dir <DIR>` - Specify cache directory (default: `.loopsleuth_cache`)

**Note**:
- The tool shows a real-time progress bar with function names and status
- Cached results are shown with a 💾 icon for instant retrieval
- For extremely large functions (>500 lines), consider using `--skip-large N`
- If you get "Function too large" warnings, increase `--context-size` to 8192 or higher

## Example

```bash
# List all available checks
loopsleuth --list-checks

# Print default configuration
loopsleuth --print-default-config > my-loopsleuth.toml

# Run with custom configuration
loopsleuth --config my-loopsleuth.toml -m ~/.loopsleuth/models/qwen*.gguf ./test_examples/sample.py

# Run all checks (default)
loopsleuth -m ~/.loopsleuth/models/qwen*.gguf ./test_examples/sample.py

# Run specific checks only
loopsleuth -m ~/.loopsleuth/models/qwen*.gguf ./test_examples/sample.py --checks quadratic,linear-in-loop

# Run all except ML-specific checks
loopsleuth -m ~/.loopsleuth/models/qwen*.gguf ./test_examples/sample.py --exclude conversion-churn,ml-footguns

# Full analysis in terminal
loopsleuth -m ~/.loopsleuth/models/qwen*.gguf ./test_examples/sample.py --details

# Save detailed report to file
loopsleuth -m ~/.loopsleuth/models/qwen*.gguf ./test_examples/sample.py --output report.md
```

**For developers**: If you're building from source, use `cargo run --release --` instead of `loopsleuth`, or use `make example`.

## Output Format

LoopSleuth provides **flexible output** for different use cases:

### Default: Concise Summary
A quick overview showing:
- Total functions analyzed
- Checks run
- Count of functions with issues (any check)
- List of issues grouped by function

**Perfect for:** Quick checks, CI/CD pipelines, daily development

### With --details: Full Report to stdout
Each function with issues includes:
- 📝 Full source code
- ⚠️ Analysis for each detected issue
- 💡 Optimization suggestions with examples for each issue

**Perfect for:** Deep analysis, learning, immediate review

### With --output FILE: Save Markdown Report
Generate a complete markdown file that can be:
- Committed to your repository
- Attached to pull requests
- Shared in code reviews
- Used as documentation

**Note:** File output always includes full details regardless of `--details` flag

Sample output:
```
🔧 Initializing LoopSleuth...
   ⚙️  Setting up LLM backend...
   📦 Loading model: ./models/qwen2.5-coder-3b-instruct-q4_k_m.gguf...
   ✅ Ready! (context: 4096 tokens)

🔍 Scanning 1 Python file(s)...
🔬 Running 3 check(s): quadratic, linear-in-loop, unbounded-alloc
📊 Analyzing 4 function(s)...

[██████████████████████████████] 100% [4/4] | Issues: 3 | 🔍 [unbounded-alloc] clean_function
✅ Analysis complete!

╔═══════════════════════════════╗
║ LOOPSLEUTH ANALYSIS SUMMARY   ║
╚═══════════════════════════════╝

📊 Total functions analyzed: 4
🔍 Checks run: 3 (quadratic, linear-in-loop, unbounded-alloc)
⚠️  Functions with issues: 3
✓  Functions clean: 1
💾 Cache entries: 12 (expected: 12 = 4 functions × 3 checks), 8 with issues

🔴 ISSUES DETECTED:
─────────────────────────────────────────────────────────────
  • quadratic_example (test.py:1)
    - Quadratic Complexity
    - Linear Operations in Loops
    - Unbounded Allocations
  • linear_in_loop_example (test.py:10)
    - Quadratic Complexity
    - Linear Operations in Loops
  • string_concat_example (test.py:17)
    - Quadratic Complexity
    - Linear Operations in Loops
    - Unbounded Allocations

💡 Tip: Use --details to see full analysis or --output FILE to save report
```

## How It Works

1. **File Discovery**: Walks through the specified path to find all `.py` files
2. **Parsing**: Uses RustPython's parser to build an AST
3. **Function Extraction**: Extracts all function definitions (including class methods)
4. **Check Selection**: Determines which checks to run based on CLI flags (default: all 8 checks)
5. **For each function, run all selected checks**:
   - **Cache Check**: Computes SHA256 hash of function source code + check key and checks SQLite cache
     - **Cache Hit**: Instantly returns cached analysis results (shown with 💾 icon)
     - **Cache Miss**: Proceeds to LLM analysis
   - **Two-Stage LLM Analysis** (per check, when not in cache):
     - **Stage 1 - Detection**: Constructs a check-specific prompt asking the LLM to analyze for that issue
     - Runs inference using llama.cpp to identify the issue
     - **Stage 2 - Solution**: If issue detected, makes a second LLM call to:
       - Explain why the code has this issue
       - Propose specific optimization strategies
       - Provide optimized code examples
     - **Cache Storage**: Stores analysis results in SQLite with composite key (function_hash, check_key)
6. **Reporting**: Displays findings grouped by function, showing all detected issues with solutions

### Caching Benefits

The intelligent caching system provides significant benefits:

- **Speed**: Instant results for unchanged functions (no LLM calls needed)
- **Cost**: Saves computation time on repeated analyses
- **Consistency**: Same function always gets same analysis (deterministic)
- **Automatic Invalidation**: Cache key is based on function source code hash - any code change automatically invalidates cache entry
- **Persistent**: Cache survives across runs (stored in `.loopsleuth_cache/` by default)
- **Zero Configuration**: Works automatically - just run the tool

**Example speed improvement:**
- First run on 100 functions with 8 checks: ~40-60 minutes
- Second run (all cached): ~10-20 seconds
- Incremental run (95% cached): ~2-5 minutes
- Single check (quadratic only): ~5-8 minutes first run, instant when cached

**Cache behavior:**
- Results cached per (function, check) combination
- Functions identified by SHA256 hash of source code
- Changing even a single character in a function invalidates its cache entries for all checks
- Cache automatically migrates from old single-check schema to new multi-check schema
- Cache is stored in SQLite database (`.loopsleuth_cache/analysis_cache.db`)
- Cache statistics shown in summary: "💾 Cache entries: X (expected: Y = N functions × M checks), Z with issues"

## Common Patterns Detected

### Performance Issues
- **Quadratic complexity**: Nested loops, repeated linear operations
- **Linear-in-loop**: `x in list`, `.remove()`, `.index()`, `.pop(0)` in loops
- **N+1 problem**: File I/O, network calls, model loading in loops
- **Expensive sort keys**: O(n) key functions in sorting
- **Unbounded allocations**: String concatenation, repeated concatenation in loops
- **Growing containers**: Appending to lists while iterating

### ML-Specific Issues
- **Conversion churn**: Repeated `.cpu()`, `.cuda()`, `.numpy()` conversions
- **ML anti-patterns**: Repeated tokenization, mask rebuilding, Python loops over tensors

## Model Recommendations

| Model | Size | Speed | Accuracy | Best For |
|-------|------|-------|----------|----------|
| **Qwen2.5-Coder (3B)** ⭐ | ~2GB | Fast | Excellent | **Recommended** - Code-specific training |
| Devstral Small 2 (24B) | ~15GB | Slower | Excellent | Production, very detailed analysis |
| Qwen2.5 (3B) | ~2GB | Fast | Good | General purpose |
| Qwen2.5 (0.5B) | ~400MB | Very Fast | Fair | Quick checks, testing |

## Performance

- Model loading: ~1-3 seconds (depending on model size)
- Per-function, per-check analysis (2 LLM calls when issue detected):
  - Detection: ~2-5 seconds
  - Solution proposal: ~3-8 seconds
  - **Cached retrieval: <10ms (instant!)**
- Running all 8 checks: ~8x time compared to single check (but only on first run - subsequent runs use cache)
- The tool processes functions sequentially to manage memory
- Larger models (24B) provide more detailed and accurate analysis but require more RAM
- **Cache dramatically improves repeated runs**: Second analysis on same codebase is ~100x faster
- **Tip**: Use `--checks` to run only the checks you need for faster first-time analysis

## Troubleshooting

### Large Functions

**Symptoms**: "Function too large" warnings for very large functions (>500 lines)

**Solution**: Increase context size to accommodate larger functions
```bash
loopsleuth --context-size 8192 -m ~/.loopsleuth/models/qwen*.gguf ./code
```

Or skip analyzing extremely large functions:
```bash
loopsleuth --skip-large 300 -m ~/.loopsleuth/models/qwen*.gguf ./code
```

### Slow Analysis

**Symptoms**: Takes a while to analyze many functions

**This is normal**:
- Each function requires 2 LLM calls per check (detection + solution) if issue found
- With all 8 checks: expect ~40-80 seconds per function on first run (depending on issues found)
- With single check: expect ~5-10 seconds per function with 3B model
- Progress bar shows real-time status with check name and function name
- Second run is instant if code hasn't changed (cache hit)

**To speed up**:
- Use `--checks` to run only needed checks (e.g., `--checks quadratic,linear-in-loop`)
- Use `--exclude` to skip ML-specific checks if not relevant
- Use smaller models (Qwen2.5-0.5B) for faster analysis at cost of accuracy
- Use `--skip-large` to skip very large functions
- Let the cache work - subsequent runs are ~100x faster

### Out of Memory

**Symptoms**: System runs out of RAM (rare with default settings)

**Solutions**:
- Use smaller model (Qwen2.5-0.5B instead of 3B)
- Close other memory-intensive applications

## License

MIT
