use anyhow::{Context, Result};
use clap::Parser;
use llama_cpp_2::context::params::LlamaContextParams;
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::LlamaModel;
use llama_cpp_2::context::LlamaContext;
use rustpython_parser::{parse, Mode};
use rustpython_ast::{Mod, Stmt};
use std::num::NonZeroU32;
use std::path::PathBuf;
use std::fs::OpenOptions;
use std::os::unix::io::AsRawFd;
use walkdir::WalkDir;
use rusqlite::{Connection, params};
use sha2::{Sha256, Digest};
use std::fs;

#[derive(Parser)]
#[command(name = "loopsleuth")]
#[command(about = "Detect quadratic complexity in Python code using LLM analysis", long_about = None)]
struct Cli {
    /// Path to the Python module or file to analyze
    #[arg(value_name = "PATH")]
    python_path: PathBuf,

    /// Path to the GGUF model file
    #[arg(short, long, value_name = "MODEL")]
    model: PathBuf,

    /// Number of threads to use for inference
    #[arg(short, long, default_value_t = 4)]
    threads: u32,

    /// Maximum tokens to generate
    #[arg(long, default_value_t = 512)]
    max_tokens: i32,

    /// Context size (max tokens for input + output)
    #[arg(long, default_value_t = 4096)]
    context_size: u32,

    /// Show verbose llama.cpp output
    #[arg(short, long)]
    verbose: bool,

    /// Output report to file (markdown format)
    #[arg(short, long, value_name = "FILE")]
    output: Option<PathBuf>,

    /// Show detailed report in stdout (always included in --output file)
    #[arg(short, long)]
    details: bool,

    /// Skip functions larger than this many lines (0 = no limit)
    #[arg(long, default_value_t = 0)]
    skip_large: usize,

    /// Disable caching of analysis results
    #[arg(long)]
    no_cache: bool,

    /// Clear the cache before running analysis
    #[arg(long)]
    clear_cache: bool,

    /// Directory for cache storage (default: .loopsleuth_cache)
    #[arg(long, value_name = "DIR")]
    cache_dir: Option<PathBuf>,
}

#[derive(Clone)]
struct FunctionInfo {
    name: String,
    source: String,
    file_path: PathBuf,
    line_number: usize,
}

#[derive(Clone)]
struct AnalysisResult {
    function: FunctionInfo,
    is_quadratic: bool,
    analysis: String,
    solution: Option<String>,
}

struct FileResults {
    file_path: PathBuf,
    results: Vec<AnalysisResult>,
}

/// Cache for storing LLM analysis results
struct AnalysisCache {
    conn: Connection,
    enabled: bool,
}

#[derive(Debug)]
struct CachedResult {
    is_quadratic: bool,
    analysis: String,
    solution: Option<String>,
}

impl AnalysisCache {
    /// Create or open cache database
    fn new(cache_dir: Option<PathBuf>, enabled: bool) -> Result<Self> {
        if !enabled {
            // Return a dummy cache with an in-memory database
            return Ok(Self {
                conn: Connection::open_in_memory()?,
                enabled: false,
            });
        }

        let cache_dir = cache_dir.unwrap_or_else(|| PathBuf::from(".loopsleuth_cache"));

        // Create cache directory if it doesn't exist
        fs::create_dir_all(&cache_dir)
            .context("Failed to create cache directory")?;

        let db_path = cache_dir.join("analysis_cache.db");
        let conn = Connection::open(&db_path)
            .context("Failed to open cache database")?;

        // Create table if it doesn't exist
        conn.execute(
            "CREATE TABLE IF NOT EXISTS analysis_results (
                function_hash TEXT PRIMARY KEY,
                is_quadratic INTEGER NOT NULL,
                analysis TEXT NOT NULL,
                solution TEXT,
                created_at INTEGER NOT NULL
            )",
            [],
        )?;

        Ok(Self {
            conn,
            enabled: true,
        })
    }

    /// Compute SHA256 hash of function source code
    fn hash_function(source: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(source.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Check if analysis result exists in cache
    fn get(&self, func: &FunctionInfo) -> Result<Option<CachedResult>> {
        if !self.enabled {
            return Ok(None);
        }

        let hash = Self::hash_function(&func.source);

        let mut stmt = self.conn.prepare(
            "SELECT is_quadratic, analysis, solution FROM analysis_results WHERE function_hash = ?1"
        )?;

        let result = stmt.query_row(params![hash], |row| {
            Ok(CachedResult {
                is_quadratic: row.get::<_, i32>(0)? != 0,
                analysis: row.get(1)?,
                solution: row.get(2)?,
            })
        });

        match result {
            Ok(cached) => Ok(Some(cached)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    /// Store analysis result in cache
    fn put(&self, func: &FunctionInfo, is_quadratic: bool, analysis: &str, solution: Option<&str>) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let hash = Self::hash_function(&func.source);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        self.conn.execute(
            "INSERT OR REPLACE INTO analysis_results (function_hash, is_quadratic, analysis, solution, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5)",
            params![hash, is_quadratic as i32, analysis, solution, timestamp],
        )?;

        Ok(())
    }

    /// Clear all cache entries
    fn clear(&self) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        self.conn.execute("DELETE FROM analysis_results", [])?;
        Ok(())
    }

    /// Get cache statistics
    fn stats(&self) -> Result<(usize, usize)> {
        if !self.enabled {
            return Ok((0, 0));
        }

        let total: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM analysis_results",
            [],
            |row| row.get(0)
        )?;

        let quadratic: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM analysis_results WHERE is_quadratic = 1",
            [],
            |row| row.get(0)
        )?;

        Ok((total, quadratic))
    }
}

/// RAII guard that redirects stderr to /dev/null and restores it on drop
struct StderrSuppressor {
    original_stderr: Option<i32>,
}

impl StderrSuppressor {
    fn new() -> Result<Self> {
        unsafe {
            // Duplicate stderr so we can restore it later
            let original_stderr = libc::dup(libc::STDERR_FILENO);
            if original_stderr < 0 {
                return Err(anyhow::anyhow!("Failed to duplicate stderr"));
            }

            // Open /dev/null
            let devnull = OpenOptions::new()
                .write(true)
                .open("/dev/null")?;

            // Redirect stderr to /dev/null
            if libc::dup2(devnull.as_raw_fd(), libc::STDERR_FILENO) < 0 {
                return Err(anyhow::anyhow!("Failed to redirect stderr"));
            }

            Ok(StderrSuppressor {
                original_stderr: Some(original_stderr),
            })
        }
    }
}

impl Drop for StderrSuppressor {
    fn drop(&mut self) {
        if let Some(original) = self.original_stderr {
            unsafe {
                // Restore original stderr
                libc::dup2(original, libc::STDERR_FILENO);
                libc::close(original);
            }
        }
    }
}

fn main() -> Result<()> {
    // Set up panic hook to provide better error messages
    std::panic::set_hook(Box::new(|panic_info| {
        eprintln!("\n❌ Fatal error occurred!");
        if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
            eprintln!("   Error: {}", s);
        }
        if let Some(location) = panic_info.location() {
            eprintln!("   Location: {}:{}", location.file(), location.line());
        }
        eprintln!("\n💡 Try using --context-size 8192 or larger if analyzing big functions");
        eprintln!("   Or set RUST_BACKTRACE=1 for full backtrace\n");
    }));

    let cli = Cli::parse();

    // Show initialization message (before suppressor)
    println!("🔧 Initializing LoopSleuth...");

    // Suppress llama.cpp logs unless verbose mode is enabled
    // Keep the suppressor active for the entire run
    let _suppressor = if !cli.verbose {
        println!("   ⚙️  Setting up LLM backend...");
        println!("   📦 Loading model: {}...", cli.model.display());
        Some(StderrSuppressor::new()?)
    } else {
        None
    };

    // Initialize llama backend
    let backend = LlamaBackend::init()?;

    // Load model
    let model_params = LlamaModelParams::default();
    let model = LlamaModel::load_from_file(&backend, &cli.model, &model_params)
        .context("Failed to load model")?;

    // Create context with configurable size
    let n_ctx = NonZeroU32::new(cli.context_size)
        .context("Invalid context size")?;
    let ctx_params = LlamaContextParams::default()
        .with_n_ctx(Some(n_ctx))
        .with_n_batch(4096)
        .with_n_threads(cli.threads as i32);

    let mut ctx = model.new_context(&backend, ctx_params)
        .context("Failed to create context")?;

    println!("   ✅ Ready! (context: {} tokens)\n", cli.context_size);

    // Initialize cache
    let cache = AnalysisCache::new(cli.cache_dir.clone(), !cli.no_cache)?;

    // Clear cache if requested
    if cli.clear_cache {
        println!("🗑️  Clearing cache...");
        cache.clear()?;
    }

    // Collect Python files
    let python_files = collect_python_files(&cli.python_path)?;
    let file_count = python_files.len();

    println!("🔍 Scanning {} Python file(s)...", file_count);

    // First pass: count total functions
    let mut total_functions_count = 0;
    for path in &python_files {
        if let Ok(functions) = extract_functions(path) {
            total_functions_count += functions.len();
        }
    }

    println!("📊 Analyzing {} function(s)...\n", total_functions_count);

    let mut all_file_results: Vec<FileResults> = Vec::new();
    let mut total_functions = 0;
    let mut current_func_num = 0;
    let mut quadratic_count = 0;

    // Process each file
    for file_path in &python_files {
        let functions = extract_functions(&file_path)?;
        let mut file_results = Vec::new();

        for func in functions {
            total_functions += 1;

            current_func_num += 1;

            // Calculate progress bar (for all messages)
            let progress_pct = (current_func_num as f32 / total_functions_count as f32 * 100.0) as usize;
            let bar_width = 30;
            let filled = (current_func_num as f32 / total_functions_count as f32 * bar_width as f32) as usize;
            let empty = bar_width - filled;
            let progress_bar = format!("[{}{}]", "█".repeat(filled), "░".repeat(empty));

            // Get filename for display
            let filename = file_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");
            let func_display = format!("{}::{}", filename, func.name);

            // Skip large functions if requested
            if cli.skip_large > 0 {
                let line_count = func.source.lines().count();
                if line_count > cli.skip_large {
                    // Update display and continue
                    print!("\r\x1b[K{} {}% [{}/{}] | Quadratic: {} | ⊗ Skipped: {} (too large)",
                           progress_bar, progress_pct, current_func_num, total_functions_count,
                           quadratic_count, func_display);
                    std::io::Write::flush(&mut std::io::stdout()).ok();
                    continue;
                }
            }

            // Check cache first
            if let Ok(Some(cached)) = cache.get(&func) {
                // Cache hit! Use cached results
                if cached.is_quadratic {
                    quadratic_count += 1;
                }

                print!("\r\x1b[K{} {}% [{}/{}] | Quadratic: {} | 💾 Cached: {}",
                       progress_bar, progress_pct, current_func_num, total_functions_count,
                       quadratic_count, func_display);
                std::io::Write::flush(&mut std::io::stdout()).ok();

                file_results.push(AnalysisResult {
                    function: func,
                    is_quadratic: cached.is_quadratic,
                    analysis: cached.analysis,
                    solution: cached.solution,
                });

                continue;
            }

            // Cache miss - run LLM analysis
            print!("\r\x1b[K{} {}% [{}/{}] | Quadratic: {} | 🔍 Analyzing: {}",
                   progress_bar, progress_pct, current_func_num, total_functions_count,
                   quadratic_count, func_display);
            std::io::Write::flush(&mut std::io::stdout()).ok();

            // Debug: Show which function we're about to analyze
            if cli.verbose {
                eprintln!("\nDEBUG: About to analyze {} from {}", func.name, func.file_path.display());
            }

            // Wrap in catch_unwind to prevent aborts
            let analysis_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                analyze_complexity(&model, &mut ctx, &func, cli.max_tokens)
            }));

            if cli.verbose {
                eprintln!("DEBUG: Finished analyzing {}", func.name);
            }

            let analysis_result = match analysis_result {
                Ok(res) => res,
                Err(_) => {
                    print!("\r\x1b[K{} {}% [{}/{}] | Quadratic: {} | 💥 Error: {} (panic caught)",
                           progress_bar, progress_pct, current_func_num, total_functions_count,
                           quadratic_count, func_display);
                    std::io::Write::flush(&mut std::io::stdout()).ok();
                    continue;
                }
            };

            match analysis_result {
                Ok(analysis) => {
                    let is_quadratic = is_quadratic_detected(&analysis);
                    if is_quadratic {
                        quadratic_count += 1;

                        // Show that we're generating solution
                        print!("\r\x1b[K{} {}% [{}/{}] | Quadratic: {} | 💡 Generating solution...",
                               progress_bar, progress_pct, current_func_num, total_functions_count, quadratic_count);
                        std::io::Write::flush(&mut std::io::stdout()).ok();

                        // Get optimization suggestion (also wrapped to prevent aborts)
                        let solution = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            propose_solution(&model, &mut ctx, &func, cli.max_tokens)
                        }))
                        .ok()
                        .and_then(|r| r.ok());

                        // Store in cache
                        let _ = cache.put(&func, true, &analysis, solution.as_deref());

                        file_results.push(AnalysisResult {
                            function: func,
                            is_quadratic: true,
                            analysis,
                            solution,
                        });
                    } else {
                        // Store in cache
                        let _ = cache.put(&func, false, &analysis, None);

                        file_results.push(AnalysisResult {
                            function: func,
                            is_quadratic: false,
                            analysis,
                            solution: None,
                        });
                    }
                }
                Err(e) => {
                    // Show warning for skipped functions
                    let error_msg = e.to_string();
                    print!("\r\x1b[K{} {}% [{}/{}] | Quadratic: {} | ⚠️  {}",
                           progress_bar, progress_pct, current_func_num, total_functions_count,
                           quadratic_count, if error_msg.contains("too large") { "Function too large" } else { "Error" });
                    std::io::Write::flush(&mut std::io::stdout()).ok();
                }
            }
        }

        if !file_results.is_empty() {
            all_file_results.push(FileResults {
                file_path: file_path.clone(),
                results: file_results,
            });
        }
    }

    // Clear the progress line and show completion
    print!("\r\x1b[K");
    println!("✅ Analysis complete!\n");

    // Flatten results for compatibility
    let all_results: Vec<AnalysisResult> = all_file_results
        .iter()
        .flat_map(|fr| fr.results.iter())
        .cloned()
        .collect();

    let quadratic_results: Vec<_> = all_results.iter().filter(|r| r.is_quadratic).collect();
    let quadratic_count = quadratic_results.len();

    // Print concise summary
    print_summary(&all_file_results, file_count, total_functions, quadratic_count, &cache, cli.no_cache);

    // Print detailed markdown report only if --details flag is set
    if quadratic_count > 0 && cli.details {
        print_detailed_report(&quadratic_results);
    } else if quadratic_count > 0 && !cli.details && cli.output.is_none() {
        println!("💡 Tip: Use --details to see full analysis or --output FILE to save report");
        println!();
    }

    // Save to file if requested (always includes full details)
    if let Some(output_path) = &cli.output {
        write_report_to_file(output_path, &all_results, &quadratic_results, total_functions, quadratic_count, &cache, cli.no_cache)?;
        println!("📄 Report saved to: {}", output_path.display());
    }

    Ok(())
}

fn collect_python_files(path: &PathBuf) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();

    if path.is_file() {
        if path.extension().and_then(|s| s.to_str()) == Some("py") {
            files.push(path.clone());
        }
    } else if path.is_dir() {
        for entry in WalkDir::new(path)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("py"))
        {
            files.push(entry.path().to_path_buf());
        }
    }

    Ok(files)
}

fn extract_functions(file_path: &PathBuf) -> Result<Vec<FunctionInfo>> {
    let source = std::fs::read_to_string(file_path)
        .context("Failed to read Python file")?;

    let parsed = parse(&source, Mode::Module, "<embedded>")
        .map_err(|e| anyhow::anyhow!("Failed to parse Python: {:?}", e))?;

    let mut functions = Vec::new();

    if let Mod::Module(module) = parsed {
        extract_functions_from_body(&module.body, &source, file_path, &mut functions);
    }

    Ok(functions)
}

fn extract_functions_from_body(
    body: &[Stmt],
    source: &str,
    file_path: &PathBuf,
    functions: &mut Vec<FunctionInfo>,
) {
    for stmt in body {
        match stmt {
            Stmt::FunctionDef(func_def) => {
                let func_source = extract_source_from_range(&source, func_def.range.start(), func_def.range.end());
                let line_number = count_lines_to_offset(&source, func_def.range.start());

                functions.push(FunctionInfo {
                    name: func_def.name.to_string(),
                    source: func_source,
                    file_path: file_path.clone(),
                    line_number,
                });
            }
            Stmt::AsyncFunctionDef(func_def) => {
                let func_source = extract_source_from_range(&source, func_def.range.start(), func_def.range.end());
                let line_number = count_lines_to_offset(&source, func_def.range.start());

                functions.push(FunctionInfo {
                    name: func_def.name.to_string(),
                    source: func_source,
                    file_path: file_path.clone(),
                    line_number,
                });
            }
            Stmt::ClassDef(class_def) => {
                // Recursively extract functions from class bodies
                extract_functions_from_body(&class_def.body, source, file_path, functions);
            }
            _ => {}
        }
    }
}

fn extract_source_from_range(source: &str, start: impl Into<usize>, end: impl Into<usize>) -> String {
    let start_usize: usize = start.into();
    let end_usize: usize = end.into();
    source.get(start_usize..end_usize)
        .unwrap_or("")
        .to_string()
}

fn count_lines_to_offset(source: &str, offset: impl Into<usize>) -> usize {
    let offset_usize: usize = offset.into();
    source[..offset_usize.min(source.len())]
        .lines()
        .count()
        + 1
}

fn analyze_complexity(
    model: &LlamaModel,
    ctx: &mut LlamaContext,
    func: &FunctionInfo,
    max_tokens: i32,
) -> Result<String> {
    let prompt = format!(
        r#"<|im_start|>system
You are a code complexity analyzer. Your task is to analyze Python functions and identify if they contain quadratic O(n²) or worse time complexity patterns.

Common quadratic patterns include:
- Nested loops iterating over the same or related data structures
- Loop with inner O(n) operations (like list.remove(), list.index(), string concatenation)
- Repeated linear searches within a loop
- Naive sorting or comparison algorithms

Respond with "QUADRATIC" if you detect O(n²) or worse complexity, followed by a brief explanation.
Respond with "OK" if the complexity is better than quadratic.<|im_end|>
<|im_start|>user
Analyze the time complexity of this Python function:

```python
{}
```

Is this function quadratic (O(n²)) or worse?<|im_end|>
<|im_start|>assistant
"#,
        func.source
    );

    generate_response(model, ctx, &prompt, max_tokens)
}

fn propose_solution(
    model: &LlamaModel,
    ctx: &mut LlamaContext,
    func: &FunctionInfo,
    max_tokens: i32,
) -> Result<String> {
    let prompt = format!(
        r#"<|im_start|>system
You are an expert Python performance optimization consultant. Your task is to provide concrete, actionable solutions to fix quadratic complexity in Python functions.

Provide:
1. A brief explanation of why the current code is O(n²)
2. A specific optimization strategy (e.g., use set/dict for O(1) lookup, list comprehension, built-in functions, better algorithm)
3. A code example showing the optimized version<|im_end|>
<|im_start|>user
This Python function has O(n²) complexity:

```python
{}
```

How can this be optimized to have better time complexity? Provide a specific solution with code.<|im_end|>
<|im_start|>assistant
"#,
        func.source
    );

    generate_response(model, ctx, &prompt, max_tokens)
}

fn generate_response(
    model: &LlamaModel,
    ctx: &mut LlamaContext,
    prompt: &str,
    max_tokens: i32,
) -> Result<String> {
    // Tokenize the prompt (AddBos::Always adds BOS token)
    let tokens = model.str_to_token(prompt, llama_cpp_2::model::AddBos::Always)?;

    // Get context size from context
    let ctx_size = ctx.n_ctx() as usize;

    // Reserve space for response tokens - need prompt + response + safety margin
    let safety_margin = 100; // Extra tokens for safety
    let max_prompt_size = ctx_size.saturating_sub(max_tokens as usize).saturating_sub(safety_margin);

    // Check if prompt is too large
    if tokens.len() > max_prompt_size {
        return Err(anyhow::anyhow!(
            "Function too large ({} tokens, context allows {}). Use --context-size {} or --skip-large.",
            tokens.len(),
            max_prompt_size,
            ctx_size * 2
        ));
    }

    // Clear context and add tokens
    ctx.clear_kv_cache();

    // Use larger batch size to accommodate big functions
    let mut batch = LlamaBatch::new(4096, 1);

    // Dynamically adjust max_tokens if prompt is large to avoid context overflow
    let available_tokens = ctx_size.saturating_sub(tokens.len()).saturating_sub(safety_margin);
    let actual_max_tokens = max_tokens.min(available_tokens as i32);

    // Add all tokens to the batch. Only request logits for the last token
    for (i, token) in tokens.iter().enumerate() {
        let is_last = i == tokens.len() - 1;
        batch.add(*token, i as i32, &[0], is_last)?;
    }

    ctx.decode(&mut batch)?;

    // Generate response
    let mut response = String::new();
    let mut n_cur = tokens.len() as i32;

    for _ in 0..actual_max_tokens {
        let mut candidates: Vec<_> = ctx.candidates().collect();

        if candidates.is_empty() {
            break;
        }

        // Sort by probability (descending) for greedy sampling
        candidates.sort_by(|a, b| b.logit().partial_cmp(&a.logit()).unwrap());

        // Greedy sampling - pick the token with highest probability
        let new_token = candidates[0].id();

        if model.is_eog_token(new_token) {
            break;
        }

        // Convert token to string
        let token_str = model.token_to_str(new_token, llama_cpp_2::model::Special::Tokenize)?;
        response.push_str(&token_str);

        // Prepare next batch
        batch.clear();
        batch.add(new_token, n_cur, &[0], true)?;
        ctx.decode(&mut batch)?;

        n_cur += 1;
    }

    Ok(response)
}

fn is_quadratic_detected(analysis: &str) -> bool {
    let analysis_lower = analysis.to_lowercase();
    analysis_lower.contains("quadratic") || analysis_lower.contains("o(n²)") || analysis_lower.contains("o(n^2)")
}

fn print_summary(file_results: &[FileResults], file_count: usize, total: usize, quadratic_count: usize, cache: &AnalysisCache, no_cache: bool) {
    println!("\n╔═════════════════════════════╗");
    println!("║ LOOPSLEUTH ANALYSIS SUMMARY ║");
    println!("╚═════════════════════════════╝");
    println!();

    if file_count > 1 {
        println!("📁 Files analyzed: {}", file_count);
    }
    println!("📊 Total functions analyzed: {}", total);
    println!("⚠️  Functions with O(n²) complexity: {}", quadratic_count);
    println!("✓  Functions OK: {}", total - quadratic_count);

    // Show cache statistics if enabled
    if !no_cache {
        if let Ok((cache_total, cache_quadratic)) = cache.stats() {
            if cache_total > 0 {
                println!("💾 Cache entries: {} total, {} quadratic", cache_total, cache_quadratic);
            }
        }
    }

    if quadratic_count > 0 {
        println!("\n🔴 QUADRATIC COMPLEXITY DETECTED IN:");
        println!("─────────────────────────────────────────────────────────────");

        if file_count > 1 {
            // Group by file when analyzing multiple files
            for file_result in file_results {
                let quadratic_in_file: Vec<_> = file_result.results.iter().filter(|r| r.is_quadratic).collect();
                if !quadratic_in_file.is_empty() {
                    println!("\n  📄 {}", file_result.file_path.display());
                    for result in quadratic_in_file {
                        println!(
                            "     • {} (line {})",
                            result.function.name,
                            result.function.line_number
                        );
                    }
                }
            }
        } else {
            // Flat list for single file
            for file_result in file_results {
                for result in file_result.results.iter().filter(|r| r.is_quadratic) {
                    println!(
                        "  • {} ({}:{})",
                        result.function.name,
                        result.function.file_path.display(),
                        result.function.line_number
                    );
                }
            }
        }
    }

    println!();
}

fn print_detailed_report(quadratic_results: &[&AnalysisResult]) {
    println!("═══════════════════════════════════════════════════════════════");
    println!("                     DETAILED REPORT");
    println!("═══════════════════════════════════════════════════════════════");
    println!();

    for (idx, result) in quadratic_results.iter().enumerate() {
        println!("## {} - `{}`", idx + 1, result.function.name);
        println!();
        println!("**Location:** `{}:{}`",
            result.function.file_path.display(),
            result.function.line_number
        );
        println!();

        println!("### 📝 Original Code");
        println!();
        println!("```python");
        println!("{}", result.function.source);
        println!("```");
        println!();

        println!("### ⚠️ Analysis");
        println!();
        println!("{}", result.analysis.trim());
        println!();

        if let Some(solution) = &result.solution {
            println!("### 💡 Suggested Optimization");
            println!();
            println!("{}", solution.trim());
            println!();
        }

        if idx < quadratic_results.len() - 1 {
            println!("───────────────────────────────────────────────────────────────");
            println!();
        }
    }

    println!("═══════════════════════════════════════════════════════════════");
    println!();
    println!("📄 Copy this report to your code review or documentation!");
    println!();
}

fn write_report_to_file(
    path: &PathBuf,
    all_results: &[AnalysisResult],
    quadratic_results: &[&AnalysisResult],
    total: usize,
    quadratic_count: usize,
    cache: &AnalysisCache,
    no_cache: bool,
) -> Result<()> {
    use std::io::Write;

    let mut file = std::fs::File::create(path)?;

    // Write header
    writeln!(file, "# LoopSleuth Analysis Report")?;
    writeln!(file)?;
    writeln!(file, "Generated: {}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"))?;
    writeln!(file)?;

    // Write summary
    writeln!(file, "## Summary")?;
    writeln!(file)?;
    writeln!(file, "- **Total functions analyzed:** {}", total)?;
    writeln!(file, "- **Functions with O(n²) complexity:** {}", quadratic_count)?;
    writeln!(file, "- **Functions OK:** {}", total - quadratic_count)?;

    // Add cache statistics to report
    if !no_cache {
        if let Ok((cache_total, cache_quadratic)) = cache.stats() {
            if cache_total > 0 {
                writeln!(file, "- **Cache entries:** {} total, {} quadratic", cache_total, cache_quadratic)?;
            }
        }
    }
    writeln!(file)?;

    if quadratic_count > 0 {
        writeln!(file, "## Quadratic Complexity Detected")?;
        writeln!(file)?;

        for result in all_results.iter().filter(|r| r.is_quadratic) {
            writeln!(
                file,
                "- `{}` ({}:{})",
                result.function.name,
                result.function.file_path.display(),
                result.function.line_number
            )?;
        }
        writeln!(file)?;

        // Write detailed report
        writeln!(file, "---")?;
        writeln!(file)?;
        writeln!(file, "## Detailed Analysis")?;
        writeln!(file)?;

        for (idx, result) in quadratic_results.iter().enumerate() {
            writeln!(file, "### {} - `{}`", idx + 1, result.function.name)?;
            writeln!(file)?;
            writeln!(
                file,
                "**Location:** `{}:{}`",
                result.function.file_path.display(),
                result.function.line_number
            )?;
            writeln!(file)?;

            writeln!(file, "#### 📝 Original Code")?;
            writeln!(file)?;
            writeln!(file, "```python")?;
            writeln!(file, "{}", result.function.source)?;
            writeln!(file, "```")?;
            writeln!(file)?;

            writeln!(file, "#### ⚠️ Analysis")?;
            writeln!(file)?;
            writeln!(file, "{}", result.analysis.trim())?;
            writeln!(file)?;

            if let Some(solution) = &result.solution {
                writeln!(file, "#### 💡 Suggested Optimization")?;
                writeln!(file)?;
                writeln!(file, "{}", solution.trim())?;
                writeln!(file)?;
            }

            if idx < quadratic_results.len() - 1 {
                writeln!(file, "---")?;
                writeln!(file)?;
            }
        }
    }

    writeln!(file)?;
    writeln!(file, "---")?;
    writeln!(file)?;
    writeln!(file, "*Generated by [LoopSleuth](https://github.com/tarekziade/loopsleuth)*")?;

    Ok(())
}
