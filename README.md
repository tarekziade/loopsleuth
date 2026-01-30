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
- `-v, --verbose` - Show verbose llama.cpp output (useful for debugging)
- `-o, --output <FILE>` - Save analysis report to markdown file
- `-d, --details` - Show detailed report in stdout (always included in file output)

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
Initializing LLM backend...
Loading model from "./models/model.gguf"...
Found 2 Python file(s) to analyze

Analyzing: test_examples/sample.py
  Checking function: bubble_sort (line 1)
    ⚠️  QUADRATIC COMPLEXITY DETECTED
    Analysis: QUADRATIC - Contains nested loops iterating over the same array
    💡 Generating optimization suggestion...
    Suggested fix:
      Use Python's built-in sorted() function or list.sort() which uses
      Timsort algorithm with O(n log n) complexity:

      def bubble_sort(arr):
          return sorted(arr)

      Or if you need in-place sorting:

      def bubble_sort(arr):
          arr.sort()
          return arr

  Checking function: linear_search (line 40)
    ✓ No quadratic complexity detected

=== Summary ===
Total functions analyzed: 2
Functions with quadratic complexity: 1
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

## License

MIT
