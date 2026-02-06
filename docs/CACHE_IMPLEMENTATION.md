# SQLite Cache Implementation

## Overview

LoopSleuth uses a local SQLite-based caching system with **multi-check support** to avoid redundant LLM analysis calls for unchanged functions. The cache uses SHA256 hashing combined with check keys to store results per (function, check) combination, providing significant performance improvements on repeated runs.

## Multi-Check Architecture

With 8 performance checks, the cache stores results separately for each check. This means:
- A single function with 8 checks generates 8 cache entries
- Changing a function invalidates all 8 cache entries for that function
- Adding/removing checks only affects those specific cache entries
- Cache hits can be partial (some checks cached, others need analysis)

## Implementation Details

### Dependencies Added

- `rusqlite = { version = "0.32", features = ["bundled"] }` - SQLite database interface
- `sha2 = "0.10"` - SHA256 hashing for function fingerprinting

### Core Components

#### AnalysisCache Struct

Located in `src/main.rs`, the cache implementation includes:

- **Database Schema** (New - Multi-Check):
  ```sql
  CREATE TABLE check_results (
    function_hash TEXT NOT NULL,         -- SHA256 of function source
    check_key TEXT NOT NULL,             -- Check identifier (e.g., "quadratic")
    has_issue INTEGER NOT NULL,          -- 0 or 1 boolean
    analysis TEXT NOT NULL,              -- LLM analysis text
    solution TEXT,                       -- Optional optimization suggestion
    created_at INTEGER NOT NULL,         -- Unix timestamp
    PRIMARY KEY (function_hash, check_key)
  )
  ```

- **Old Schema** (Auto-migrated):
  ```sql
  CREATE TABLE analysis_results (
    function_hash TEXT PRIMARY KEY,      -- SHA256 of function source
    is_quadratic INTEGER NOT NULL,       -- 0 or 1 boolean (old)
    analysis TEXT NOT NULL,
    solution TEXT,
    created_at INTEGER NOT NULL
  )
  ```

  On startup, if old schema detected:
  1. Creates new `check_results` table
  2. Migrates data from `analysis_results` with `check_key='quadratic'`
  3. Drops old `analysis_results` table

- **Key Methods**:
  - `new()` - Initialize cache database, auto-migrate if old schema
  - `migrate_schema()` - Migrates old single-check schema to multi-check
  - `get(func, check_key)` - Retrieve cached result by function hash + check key
  - `put(func, check_key, has_issue, analysis, solution)` - Store analysis result in cache
  - `clear()` - Delete all cache entries
  - `stats()` - Get cache statistics (total entries, entries with issues)
  - `hash_function()` - Compute SHA256 of function source code

#### Cache Behavior

1. **Cache Key**: Composite key of (function_hash, check_key)
   - `function_hash`: SHA256 hash of the complete function source code
   - `check_key`: Check identifier (e.g., "quadratic", "linear-in-loop")
   - Any code change (even whitespace) invalidates all check results for that function
   - Deterministic: Same (code, check) always produces same hash

2. **Cache Storage**:
   - Default location: `.loopsleuth_cache/analysis_cache.db`
   - Configurable via `--cache-dir` flag
   - Persistent across runs
   - Example: 100 functions × 8 checks = 800 cache entries

3. **Cache Flow** (per function, per check):
   ```
   Function + Check → Compute Hash → Check Cache(hash, check_key)
                                   ↓
                              Cache Hit? ─Yes→ Return cached result (💾 icon)
                                   ↓
                                  No
                                   ↓
                         Run LLM Detection → Issue Found?
                                               ↓
                                              Yes
                                               ↓
                                       Run LLM Solution
                                               ↓
                               Store in Cache(hash, check_key) → Return result
   ```

### CLI Options

Added three new command-line flags:

- `--no-cache` - Disable caching entirely (forces re-analysis)
- `--clear-cache` - Clear all cache entries before running
- `--cache-dir <DIR>` - Specify custom cache directory

### Integration Points

1. **Main Analysis Loop** (nested per function, per check)
   - Before each check: Call `cache.get(func, check.key)`
   - On cache hit: Skip LLM calls, use cached data, display with 💾 icon
   - On cache miss: Run LLM detection, optionally solution
   - After analysis: Store results with `cache.put(func, check.key, ...)`

2. **Summary Display**
   - Shows cache statistics: "💾 Cache entries: X (expected: Y = N functions × M checks), Z with issues"
   - Only displayed when caching is enabled
   - Helps verify cache is working correctly

3. **Report Generation**
   - Includes cache statistics in HTML output
   - Shows per-check cache effectiveness

## Performance Impact

### Speed Improvements

- **Cached check retrieval**: <10ms (vs 5-10 seconds for LLM call)
- **Second run on 100 functions, all 8 checks**: ~10-20 seconds (vs 40-60 minutes)
- **Incremental run (95% cached)**: ~2-5 minutes (vs 40-60 minutes)
- **Single check (quadratic) on 100 functions**: ~10 minutes first run, ~10 seconds cached

### Example Speedup

First run (cold cache, all 8 checks):
```
📊 Total functions analyzed: 100
🔍 Checks run: 8 (quadratic, linear-in-loop, n-plus-one, ...)
⚠️  Functions with issues: 60
✓  Functions clean: 40
💾 Cache entries: 800 (expected: 800 = 100 functions × 8 checks), 300 with issues
Time: ~40-60 minutes
```

Second run (warm cache, no changes):
```
📊 Total functions analyzed: 100
🔍 Checks run: 8 (quadratic, linear-in-loop, n-plus-one, ...)
⚠️  Functions with issues: 60
✓  Functions clean: 40
💾 Cache entries: 800 (expected: 800 = 100 functions × 8 checks), 300 with issues
Time: ~10-20 seconds (100x faster!)
```

Partial run (only quadratic and linear-in-loop, rest cached from previous all-checks run):
```
📊 Total functions analyzed: 100
🔍 Checks run: 2 (quadratic, linear-in-loop)
⚠️  Functions with issues: 40
✓  Functions clean: 60
💾 Cache entries: 200 (expected: 200 = 100 functions × 2 checks), 120 with issues
Time: ~10-15 seconds (all from cache!)
```

## Usage Examples

### Basic Usage (cache enabled by default)

```bash
# First run - analyzes and caches
./target/release/loopsleuth -m model.gguf ./src

# Second run - uses cache for unchanged functions
./target/release/loopsleuth -m model.gguf ./src
```

### Force Re-analysis

```bash
# Disable cache for this run
./target/release/loopsleuth --no-cache -m model.gguf ./src

# Or clear cache first
./target/release/loopsleuth --clear-cache -m model.gguf ./src
```

### Custom Cache Location

```bash
# Use project-specific cache
./target/release/loopsleuth --cache-dir ./my-project-cache -m model.gguf ./src
```

## Cache Invalidation

Cache entries are **automatically invalidated** when:
- Function source code changes (different hash → all checks invalidated)
- Function is renamed (different hash due to name in code)
- Function signature changes
- Function body changes (even comments/whitespace)

Cache entries are **NOT invalidated** when:
- File is renamed (hash only depends on function content)
- File is moved to different directory
- Other functions in same file change
- Different checks are selected (only uses cached results for requested checks)

**Note**: When a function changes, all 8 check cache entries for that function are invalidated and must be re-analyzed.

## Files Modified

1. **Cargo.toml** - Added `rusqlite` and `sha2` dependencies
2. **src/main.rs** - Added:
   - Cache struct and implementation (~140 lines)
   - CLI flags for cache control
   - Cache integration in analysis loop
   - Cache statistics in summary and reports
3. **README.md** - Documented cache feature and benefits
4. **.gitignore** - Added `.loopsleuth_cache/` to ignore list
5. **CACHE_IMPLEMENTATION.md** - This documentation

## Future Enhancements

Potential improvements:

1. **TTL-based expiration**: Add `--cache-ttl DAYS` flag to expire old entries
2. **Cache size limits**: Auto-prune when cache exceeds size threshold
3. **Cache statistics command**: `loopsleuth --cache-stats` to view cache info
4. **Export/import cache**: Share cache between team members
5. **Selective caching**: Cache only certain file patterns
6. **Cache warming**: Pre-populate cache for common patterns

## Testing

To verify the cache is working:

```bash
# Run analysis (should analyze and cache)
time ./target/release/loopsleuth -m model.gguf ./tests/checks/quadratic.py

# Run again (should use cache, much faster)
time ./target/release/loopsleuth -m model.gguf ./tests/checks/quadratic.py

# Should see 💾 icons instead of 🔍 on second run
```

## Notes

- Cache is **enabled by default** - no configuration needed
- Cache is **transparent** - same output whether cached or not
- Cache is **safe** - uses function content hash, not file metadata
- Cache is **local** - stored on disk, not shared across machines
- Cache is **fast** - SQLite is optimized for this use case
