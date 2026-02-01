# LoopSleuth Architecture

## Overview

LoopSleuth is a CLI tool that combines static analysis (AST parsing) with LLM-based semantic analysis to detect quadratic complexity patterns in Python code.

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

### 4. LLM Inference (`llama-cpp-2`)
- Local inference using GGUF quantized models
- No external API calls - runs entirely offline
- Two-stage analysis pipeline:
  1. **Detection**: Identifies O(n²) patterns
  2. **Solution**: Proposes optimizations

### 5. Report Generator
- **Three output modes**:
  - Default: Concise summary only
  - `--details`: Full analysis with code and solutions
  - `--output FILE`: Save complete markdown report
- Groups results by file when analyzing directories
- Shows file paths with line numbers (for IDE navigation)
- Progress feedback during initialization and analysis

## Analysis Pipeline

```
Python Files/Directories
    ↓
[Show Progress: Initializing]
    ↓
[Suppress stderr] ← StderrSuppressor (unless --verbose)
    ↓
[Load Model] ← llama.cpp
    ↓
[File Discovery] ← walkdir (recursive for directories)
    ↓
[Show Progress: Scanning N files]
    ↓
[For Each File]
    ↓
  [Parse AST] ← rustpython-parser
    ↓
  [Extract Functions]
    ↓
  [For Each Function]
    ↓
    [Show Progress: .]
    ↓
    [Stage 1: Detect Complexity] ← llama.cpp
    ↓
    [If Quadratic Detected]
      ↓
      [Stage 2: Propose Solution] ← llama.cpp
    ↓
[Group Results by File]
    ↓
[Print Concise Summary]
    ↓
[If --details: Print Full Report]
    ↓
[If --output: Save Markdown File]
```

## LLM Prompting Strategy

### Stage 1: Detection Prompt
- System message defines the task (complexity analysis)
- Lists common quadratic patterns to look for
- Requests "QUADRATIC" keyword in response for easy parsing
- Uses ChatML format (`<|im_start|>` tags)

### Stage 2: Solution Prompt
- Only called if quadratic complexity detected
- Requests:
  1. Explanation of why current code is O(n²)
  2. Optimization strategy
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
├── test_examples/           # Sample Python files for testing
│   ├── sample.py           # Mixed complexity functions
│   └── performance_issues.py # More edge cases
├── models/                  # GGUF models (not in git)
├── Cargo.toml              # Dependencies
├── README.md               # User documentation
├── ARCHITECTURE.md         # This file
├── setup.sh                # Interactive setup script
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
- Full markdown-formatted analysis in terminal
- Includes for each quadratic function:
  - Original source code
  - Complexity analysis
  - Optimization suggestions with code examples
- Perfect for learning and immediate review

### File Output (--output flag)
- Generates timestamped markdown file
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
- **Integration Tests**: `test_examples/` directory with known patterns
- **Manual Testing**: Run against real Python projects
- **Validation**: Compare LLM results with manual analysis

## Future Work

- Add configuration file support (`.loopsleuth.toml`)
- JSON output format for CI/CD integration
- VS Code extension for inline warnings
- Pre-commit hook support
- Caching mechanism for analyzed functions
- Support for incremental analysis (only changed files)
