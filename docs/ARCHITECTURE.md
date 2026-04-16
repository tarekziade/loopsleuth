# LoopSleuth Architecture

## Overview

LoopSleuth is a CLI tool that combines static analysis (AST parsing) with LLM-based semantic analysis to detect multiple performance issues in Python code. It uses a multi-check architecture that allows running 8 different performance checks on each function, with intelligent caching per check.

## Components

### 1. CLI Layer (`clap`)
- Parses command-line arguments
- Validates model path and Python code path
- Configures inference parameters (threads, max tokens)

### 2. Python Parser (`rustpython-parser`)
- Parses Python source files into Abstract Syntax Trees (AST)
- Handles syntax errors gracefully
- Supports all Python 3.x syntax

### 3. Function Extractor
- Walks the AST to find function definitions
- Extracts:
  - Function names
  - Source code text
  - File path and line numbers
  - Both top-level functions and class methods

### 4. Check Registry
- Defines 8 performance checks with check-specific prompts
- Each check has:
  - Unique key (e.g., "quadratic", "linear-in-loop")
  - Detection prompt function
  - Solution prompt function
  - Issue detection function (keyword matching)
- Supports filtering checks via CLI (`--checks`, `--exclude`)

### 5. LLM Inference (`llama-cpp-2`)
- Local inference using GGUF quantized models
- No external API calls - runs entirely offline
- Two-stage analysis pipeline (per check):
  1. **Detection**: Identifies specific issue using check-specific prompt
  2. **Solution**: Proposes optimizations if issue detected

### 6. Cache System (`rusqlite`)
- SQLite database with composite key: (function_hash, check_key)
- Caches results per (function, check) combination
- Automatically migrates from old single-check schema
- Statistics show cache entries per check

### 7. Report Generator
- **Three output modes**:
  - Default: Concise summary showing issues grouped by function
  - `--details`: Full analysis with code and solutions for all issues per function
  - `--output FILE`: Save complete HTML report
- Groups results by file when analyzing directories
- Shows all issues detected for each function
- Shows file paths with line numbers (for IDE navigation)
- Progress feedback during initialization and analysis (shows current check)

## Analysis Pipeline

```
Python Files/Directories
    ↓
[Parse CLI Args]
    ↓
[If --list-checks: Show Checks and Exit]
    ↓
[Get Checks to Run] ← Based on --checks or --exclude
    ↓
[Show Progress: Initializing]
    ↓
[Suppress stderr] ← StderrSuppressor (unless --verbose)
    ↓
[Load Model] ← llama.cpp
    ↓
[Initialize Cache] ← SQLite, auto-migrate if old schema
    ↓
[File Discovery] ← walkdir (recursive for directories)
    ↓
[Show Progress: Scanning N files, Running M checks]
    ↓
[For Each File]
    ↓
  [Parse AST] ← rustpython-parser
    ↓
  [Extract Functions]
    ↓
  [For Each Function]
    ↓
    [For Each Check]
      ↓
      [Compute Cache Key] ← (function_hash, check_key)
      ↓
      [Check Cache] ← SQLite lookup
      ↓
      [If Cache Hit]
        ↓
        [Show Progress: 💾 [check_key] function_name]
        ↓
        [Use Cached Result]
      ↓
      [If Cache Miss]
        ↓
        [Show Progress: 🔍 [check_key] function_name]
        ↓
        [Stage 1: Detect Issue] ← llama.cpp with check-specific prompt
        ↓
        [Parse Detection Response] ← Check for issue keyword
        ↓
        [If Issue Detected]
          ↓
          [Show Progress: 💡 [check_key] Solution...]
          ↓
          [Stage 2: Propose Solution] ← llama.cpp
        ↓
        [Store in Cache] ← SQLite with composite key
    ↓
[Group Results by File and Function]
    ↓
[Print Summary] ← Shows all issues per function
    ↓
[If --details: Print Full Report] ← All issues with solutions
    ↓
[If --output: Save HTML File] ← Complete report
```

## LLM Prompting Strategy

### Multi-Check Architecture
Each check has independent prompts tailored to detect specific issues:

#### Detection Prompts (9 checks)
1. **quadratic**: Detects O(n²) patterns, looks for "QUADRATIC" keyword
2. **linear-in-loop**: Detects `x in list`, `.remove()`, etc., looks for "LINEAR_IN_LOOP" keyword
3. **expensive-sort-key**: Detects O(n) sort key functions, looks for "EXPENSIVE_SORT_KEY" keyword
4. **unbounded-alloc**: Detects string concat/array growth in loops, looks for "UNBOUNDED_ALLOC" keyword
5. **conversion-churn**: Detects repeated tensor/device conversions, looks for "CONVERSION_CHURN" keyword
6. **python-loop-over-token-dimension**: Detects Python token loops, looks for "ML_LOOP_OVER_TOKENS" keyword
7. **mask-built-in-layer-loop**: Detects attention masks rebuilt in layer loops, looks for "ML_MASK_IN_LOOP" keyword
8. **embedding-equality-scan**: Detects exact-equality scans over embeddings/tables, looks for "EMBEDDING_EQUALITY_SCAN" keyword
9. **growing-container**: Detects growing while iterating, looks for "GROWING_CONTAINER" keyword

- Each prompt is check-specific with targeted examples
- Uses ChatML format (`<|im_start|>` tags)
- Requests specific keyword in response for easy parsing

#### Solution Prompts (9 checks)
- Only called if issue detected
- Check-specific solution strategies
- Requests:
  1. Explanation of why the code has this issue
  2. Optimization strategy specific to this check
  3. Code example of fix
- More detailed output allowed (higher token budget)

## Token Generation

Uses greedy sampling for deterministic results:
1. Tokenize prompt
2. Run model inference to get logits
3. Select token with highest probability
4. Append to output
5. Repeat until end-of-generation token or max tokens

## File Structure

```
LoopSleuth/
├── src/
│   └── main.rs              # All logic in single file for simplicity
├── python/                  # Python package
│   └── loopsleuth/
│       ├── __init__.py      # Package exports
│       ├── __main__.py      # CLI entry point
│       └── models.py        # Model download/management
├── tests/
│   ├── checks/             # Per-check example files
│   ├── golden/             # Golden expectations per check
│   ├── extra/              # Non-check-specific examples
│   ├── run_checks.py       # Golden test runner
│   └── test_regression.sh  # Wrapper for run_checks.py
├── docs/                    # Documentation
├── Cargo.toml              # Rust dependencies
├── pyproject.toml          # Python package metadata
├── setup.py                # setuptools-rust configuration
├── README.md               # User documentation
├── AGENTS.md               # Agent quick reference
└── Makefile                # Convenience commands

```

## Output Formats

### Concise Summary (Default)
- Clean, minimal output for quick scanning
- Shows:
  - File count (when analyzing directories)
  - Function counts (total, quadratic, OK)
  - List of problematic functions grouped by file
- Perfect for CI/CD pipelines and daily checks

### Detailed Report (--details flag)
- Full HTML-formatted analysis in terminal
- Includes for each quadratic function:
  - Original source code
  - Complexity analysis
  - Optimization suggestions with code examples
- Perfect for learning and immediate review

### File Output (--output flag)
- Generates timestamped HTML file
- Always includes full details regardless of --details flag
- Ready for:
  - Code review attachments
  - Pull request descriptions
  - Project documentation
  - Commit to repository

## Key Design Decisions

### Why Concise Output by Default?
- Reduces cognitive load during development
- Faster to scan for issues
- Better for automated tools/CI
- Users can opt-in to details when needed

### Why RustPython Parser?
- Pure Rust implementation (no Python runtime needed)
- Fast and accurate
- Well-maintained and up-to-date with Python syntax

### Why llama.cpp?
- Best-in-class performance for local inference
- Supports quantized models (smaller, faster)
- Cross-platform (Mac, Linux, Windows)
- No external dependencies once built

### Why Greedy Sampling?
- Deterministic results (same code → same analysis)
- Faster than sampling methods
- Sufficient for technical analysis tasks

### Why Two-Stage Analysis?
- Efficiency: Only generate solutions when needed
- Focused prompts: Each stage has a clear, specific goal
- Better token budget allocation

### Why Suppress llama.cpp Logs?
- Clean user experience by default
- llama.cpp outputs extensive debug logs to stderr
- `StderrSuppressor` uses RAII to redirect stderr to /dev/null
- `--verbose` flag restores logs for debugging
- Progress dots (`.`) provide feedback without noise

### Why Show Progress Messages?
- Model loading takes 3-10 seconds
- User needs feedback that tool is working
- Shows: initialization, model loading, file scanning
- Dots indicate per-function analysis progress

## Performance Considerations

### Bottlenecks
1. **Model Loading** (~1-3s): One-time cost per run
2. **Tokenization** (~50-100ms per function): Negligible
3. **Inference** (~2-8s per function): Main bottleneck

### Optimizations Applied
- Release build with LTO (Link Time Optimization)
- Quantized models (Q4_K_M): 4-bit weights, minimal accuracy loss
- Sequential processing: Manages memory, prevents OOM
- KV cache clearing: Ensures fresh context per function

### Future Optimizations (Not Implemented)
- Batch processing: Analyze multiple functions in parallel
- Smaller context size: Reduce memory footprint
- Speculative decoding: Speed up token generation
- Function filtering: Skip obviously simple functions (e.g., getters/setters)

## Error Handling

- **Parse Errors**: Report file and continue to next
- **Model Load Failures**: Fail fast with clear error message
- **Inference Errors**: Report function and continue to next
- **Missing Files**: Validate paths before processing

## Extension Points

### Adding New Complexity Patterns
Modify the detection prompt in `analyze_complexity()` to include new patterns.

### Supporting Other Languages
1. Replace `rustpython-parser` with appropriate parser
2. Adjust AST traversal logic in `extract_functions_from_body()`
3. Update prompts to reference the new language

### Using Different Models
- Any GGUF model compatible with llama.cpp works
- Adjust max_tokens if model has different context size
- May need to tune prompts for different model families

### Adding More Analysis Stages
Add additional function calls after `propose_solution()`:
- Generate test cases
- Estimate performance impact
- Suggest profiling commands

## Dependencies

| Crate | Purpose | Size Impact |
|-------|---------|-------------|
| rustpython-parser | Parse Python | ~2MB |
| rustpython-ast | Python AST types | ~500KB |
| llama-cpp-2 | LLM inference | ~50MB (includes llama.cpp) |
| clap | CLI parsing | ~500KB |
| walkdir | Recursive file traversal | ~100KB |
| anyhow | Error handling | ~50KB |
| libc | Low-level I/O (stderr control) | ~100KB |
| chrono | Timestamps for reports | ~200KB |

Total binary size (release): ~6.6MB (highly optimized)

## Security Considerations

- No network access required (offline-first)
- No telemetry or data collection
- User code never leaves local machine
- Model files should be from trusted sources (Hugging Face)

## Testing Strategy

- **Unit Tests**: Not currently implemented (single-file architecture)
- **Integration Tests**: `tests/run_checks.py` with `tests/checks/` and golden files
- **Manual Testing**: Run against real Python projects
- **Validation**: Compare LLM results with manual analysis

## Future Work

- Add configuration file support (`.loopsleuth.toml`)
- JSON output format for CI/CD integration
- VS Code extension for inline warnings
- Pre-commit hook support
- Caching mechanism for analyzed functions
- Support for incremental analysis (only changed files)
