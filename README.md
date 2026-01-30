# LoopSleuth

A Rust-based CLI tool that analyzes Python code for quadratic complexity patterns using local LLM inference.

## Features

- Parses Python code using Ruff's parser (fast and accurate)
- Extracts functions from Python modules
- Analyzes each function using a local LLM (llama.cpp)
- Detects O(n²) and worse complexity patterns
- Supports both single files and entire directories

## Prerequisites

1. **Rust toolchain** - Install from [rustup.rs](https://rustup.rs/)

2. **CMake** - Required for building llama.cpp:
   ```bash
   # macOS
   brew install cmake

   # Ubuntu/Debian
   sudo apt-get install cmake

   # Windows
   # Download from https://cmake.org/download/
   ```

3. **Hugging Face CLI** - For downloading models:
   ```bash
   # Standalone installer (recommended)
   curl -LsSf https://hf.co/cli/install.sh | bash

   # Or with pip
   pip install -U huggingface_hub

   # Or with homebrew
   brew install huggingface-cli
   ```

4. **GGUF Model** - Choose one of these recommended models:

   **Qwen2.5-Coder-3B (Recommended) - Optimized for code analysis:**
   ```bash
   hf download Qwen/Qwen2.5-Coder-3B-Instruct-GGUF \
     qwen2.5-coder-3b-instruct-q4_k_m.gguf \
     --local-dir ./models
   ```

   **Devstral Small 2 (24B) - Best accuracy, larger model:**
   ```bash
   hf download unsloth/Devstral-Small-2-24B-Instruct-2512-GGUF \
     Devstral-Small-2-24B-Instruct-2512-Q4_K_M.gguf \
     --local-dir ./models
   ```

   **Qwen2.5 (3B) - General purpose:**
   ```bash
   hf download Qwen/Qwen2.5-3B-Instruct-GGUF \
     qwen2.5-3b-instruct-q4_k_m.gguf \
     --local-dir ./models
   ```

   **Qwen2.5 (0.5B) - Fast, smaller model:**
   ```bash
   hf download Qwen/Qwen2.5-0.5B-Instruct-GGUF \
     qwen2.5-0.5b-instruct-q4_k_m.gguf \
     --local-dir ./models
   ```

## Installation

### Quick Setup (Recommended)

```bash
./setup.sh
```

The setup script will:
- Check prerequisites (Rust, CMake)
- Let you choose and download a model
- Build the project

### Manual Installation

```bash
# Build the project
cargo build --release

# Download a model manually
mkdir -p models
# Then download from Hugging Face (see Prerequisites section)
```

**Note**: The first build will take several minutes as it compiles llama.cpp from source. Subsequent builds are much faster.

The binary will be available at `target/release/loopsleuth`

## Usage

Analyze a single Python file:
```bash
./target/release/loopsleuth --model ./models/qwen2.5-coder-3b-instruct-q4_k_m.gguf example.py
```

Analyze an entire directory (recursive):
```bash
./target/release/loopsleuth --model ./models/qwen2.5-coder-3b-instruct-q4_k_m.gguf ./src
```

The tool automatically finds all `.py` files in subdirectories and groups results by file.

### Options

- `-m, --model <MODEL>` - Path to the GGUF model file (required)
- `-t, --threads <THREADS>` - Number of threads for inference (default: 4)
- `--max-tokens <MAX_TOKENS>` - Maximum tokens to generate (default: 512)
- `--context-size <SIZE>` - Context window size in tokens (default: 4096)
- `-v, --verbose` - Show verbose llama.cpp output (useful for debugging)
- `-o, --output <FILE>` - Save analysis report to markdown file
- `-d, --details` - Show detailed report in stdout (always included in file output)

**Note**:
- The tool shows a real-time progress bar with function names and status
- For extremely large functions (>500 lines), consider using `--skip-large N`
- If you get "Function too large" warnings, increase `--context-size` to 8192 or higher

## Example

```bash
# Quick check (concise summary only)
cargo run --release -- --model ./models/qwen2.5-coder-3b-instruct-q4_k_m.gguf ./test_examples/sample.py

# Full analysis in terminal
cargo run --release -- --model ./models/qwen2.5-coder-3b-instruct-q4_k_m.gguf ./test_examples/sample.py --details

# Save detailed report to file
cargo run --release -- --model ./models/qwen2.5-coder-3b-instruct-q4_k_m.gguf ./test_examples/sample.py --output report.md

# Or use make
make example
```

## Output Format

LoopSleuth provides **flexible output** for different use cases:

### Default: Concise Summary
A quick overview showing:
- Total functions analyzed
- Count of functions with O(n²) complexity
- Simple list of problematic functions with locations

**Perfect for:** Quick checks, CI/CD pipelines, daily development

### With --details: Full Report to stdout
Each quadratic function includes:
- 📝 Full source code
- ⚠️ Complexity analysis
- 💡 Optimization suggestions with examples

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
📊 Analyzing 11 function(s)...

  [█████░░░░░░░░░░░░░░░] 27% [3/11] 🔍 Analyzing: remove_elements...
  [█████░░░░░░░░░░░░░░░] 27% [3/11] 💡 Generating solution for: remove_elements...
  [█████░░░░░░░░░░░░░░░] 27% [3/11] ⚠️  QUADRATIC: remove_elements
  [███████░░░░░░░░░░░░░] 36% [4/11] 🔍 Analyzing: string_concatenation...
  ...

✅ Analysis complete!

╔═════════════════════════════╗
║ LOOPSLEUTH ANALYSIS SUMMARY ║
╚═════════════════════════════╝

📊 Total functions analyzed: 11
⚠️  Functions with O(n²) complexity: 5
✓  Functions OK: 6

🔴 QUADRATIC COMPLEXITY DETECTED IN:
─────────────────────────────────────────────────────────────
  • bubble_sort (test_examples/sample.py:1)
  • find_duplicates (test_examples/sample.py:11)
  • remove_elements (test_examples/sample.py:21)
  • string_concatenation (test_examples/sample.py:29)
  • nested_comparison (test_examples/sample.py:85)

💡 Tip: Use --details to see full analysis or --output FILE to save report
```

## How It Works

1. **File Discovery**: Walks through the specified path to find all `.py` files
2. **Parsing**: Uses RustPython's parser to build an AST
3. **Function Extraction**: Extracts all function definitions (including class methods)
4. **Two-Stage LLM Analysis**: For each function:
   - **Stage 1 - Detection**: Constructs a prompt asking the LLM to analyze complexity
   - Runs inference using llama.cpp to identify O(n²) patterns
   - **Stage 2 - Solution**: If quadratic complexity is detected, makes a second LLM call to:
     - Explain why the code is inefficient
     - Propose specific optimization strategies
     - Provide optimized code examples
5. **Reporting**: Displays findings with file paths, line numbers, and actionable solutions

## Common Quadratic Patterns Detected

- Nested loops over the same data structure
- Loop containing O(n) operations (list.remove(), list.index())
- String concatenation in loops
- Repeated linear searches
- Naive sorting algorithms

## Model Recommendations

| Model | Size | Speed | Accuracy | Best For |
|-------|------|-------|----------|----------|
| **Qwen2.5-Coder (3B)** ⭐ | ~2GB | Fast | Excellent | **Recommended** - Code-specific training |
| Devstral Small 2 (24B) | ~15GB | Slower | Excellent | Production, very detailed analysis |
| Qwen2.5 (3B) | ~2GB | Fast | Good | General purpose |
| Qwen2.5 (0.5B) | ~400MB | Very Fast | Fair | Quick checks, testing |

## Performance

- Model loading: ~1-3 seconds (depending on model size)
- Per-function analysis (2 LLM calls when quadratic detected):
  - Detection: ~2-5 seconds
  - Solution proposal: ~3-8 seconds
- The tool processes functions sequentially to manage memory
- Larger models (24B) provide more detailed and accurate analysis but require more RAM

## Troubleshooting

### Large Functions

**Symptoms**: "Function too large" warnings for very large functions (>500 lines)

**Solution**: Increase context size to accommodate larger functions
```bash
./target/release/loopsleuth --context-size 8192 -m model.gguf ./code
```

Or skip analyzing extremely large functions:
```bash
./target/release/loopsleuth --skip-large 300 -m model.gguf ./code
```

### Slow Analysis

**Symptoms**: Takes a while to analyze many functions

**This is normal**:
- Each function requires 2 LLM calls (detection + solution) if quadratic
- Expect ~5-10 seconds per function with 3B model
- Progress bar shows real-time status with function names

**To speed up**:
- Use smaller models (Qwen2.5-0.5B) for faster analysis at cost of accuracy
- Use `--skip-large` to skip very large functions

### Out of Memory

**Symptoms**: System runs out of RAM (rare with default settings)

**Solutions**:
- Use smaller model (Qwen2.5-0.5B instead of 3B)
- Close other memory-intensive applications

## License

MIT
