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
#[cfg(unix)]
use std::os::unix::io::AsRawFd;
use walkdir::WalkDir;
use rusqlite::{Connection, params};
use sha2::{Sha256, Digest};
use std::fs;
use serde::{Deserialize, Serialize};
use regex::Regex;
use std::time::{Duration, Instant};
use similar::{ChangeTag, TextDiff};

#[derive(Parser)]
#[command(name = "loopsleuth")]
#[command(about = "Detect performance issues in Python code using LLM analysis", long_about = None)]
struct Cli {
    /// Path to the Python module or file to analyze
    #[arg(value_name = "PATH")]
    python_path: Option<PathBuf>,

    /// Path to the GGUF model file
    #[arg(short, long, value_name = "MODEL")]
    model: Option<PathBuf>,

    /// Number of threads to use for inference
    #[arg(short, long, default_value_t = 4)]
    threads: u32,

    /// Maximum tokens to generate
    #[arg(long, default_value_t = 1024)]
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

    /// Comma-separated list of checks to run (default: all checks)
    #[arg(long, value_name = "CHECKS")]
    checks: Option<String>,

    /// List all available checks and exit
    #[arg(long)]
    list_checks: bool,

    /// Comma-separated list of checks to exclude from analysis
    #[arg(long, value_name = "CHECKS")]
    exclude: Option<String>,

    /// Path to custom checks configuration file (TOML format)
    #[arg(long, value_name = "FILE")]
    config: Option<PathBuf>,

    /// Print the default checks configuration to stdout and exit
    #[arg(long)]
    print_default_config: bool,

    /// Filter functions by name (substring match, case-insensitive)
    #[arg(short = 'k', long, value_name = "NAME")]
    filter_function: Option<String>,
}

/// Token usage statistics
#[derive(Debug, Clone, Default)]
struct TokenStats {
    input_tokens: usize,
    output_tokens: usize,
    generation_time: Duration,
}

impl TokenStats {
    fn new(input_tokens: usize, output_tokens: usize, generation_time: Duration) -> Self {
        Self {
            input_tokens,
            output_tokens,
            generation_time,
        }
    }

    fn add(&mut self, other: &TokenStats) {
        self.input_tokens += other.input_tokens;
        self.output_tokens += other.output_tokens;
        self.generation_time += other.generation_time;
    }

    fn tokens_per_second(&self) -> f64 {
        if self.generation_time.as_secs_f64() > 0.0 {
            self.output_tokens as f64 / self.generation_time.as_secs_f64()
        } else {
            0.0
        }
    }
}

#[derive(Clone)]
struct FunctionInfo {
    name: String,
    source: String,
    source_no_docstring: String,  // Version without docstring for LLM prompts
    file_path: PathBuf,
    line_number: usize,
    class_name: Option<String>,
}

/// Configuration for a single check loaded from TOML
#[derive(Debug, Clone, Serialize, Deserialize)]
struct CheckConfig {
    key: String,
    name: String,
    description: String,
    category: String,
    keyword: String,
    #[serde(default)]
    detection_rules: String,
    #[serde(default)]
    fix_recipes: String,
    detection_prompt: String,
    solution_prompt: String,
    #[serde(default = "default_verifier_prompt")]  // For backward compat
    verifier_prompt: String,
    #[serde(default)]
    guard: GuardConfig,
}

fn default_verifier_prompt() -> String {
    String::from("")  // Empty default for transition
}

#[derive(Debug, Clone)]
struct ParsedDetection {
    has_issue: bool,
    confidence: Option<f32>,
    _detail: String,  // Reserved for future use
}

#[derive(Debug, Clone)]
struct VerificationResult {
    is_valid: bool,
    reason: String,
}

/// Optional settings from config file
#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct ConfigSettings {
    model: Option<PathBuf>,
    threads: Option<u32>,
    max_tokens: Option<i32>,
    context_size: Option<u32>,
    skip_large: Option<usize>,
    cache_dir: Option<PathBuf>,
}

/// Container for all check configurations
#[derive(Debug, Deserialize, Serialize)]
struct ChecksConfig {
    #[serde(default)]
    settings: ConfigSettings,
    #[serde(default)]
    templates: std::collections::HashMap<String, String>,
    check: Vec<CheckConfig>,
    #[serde(default)]
    dedupe: Vec<DedupeRule>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct DedupeRule {
    #[serde(default)]
    prefer: String,
    #[serde(default)]
    drop: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
struct GuardConfig {
    #[serde(default)]
    require_any: Vec<String>,
    #[serde(default)]
    require_all: Vec<String>,
    #[serde(default)]
    exclude_any: Vec<String>,
    #[serde(default)]
    require_regex_any: Vec<String>,
    #[serde(default)]
    require_regex_all: Vec<String>,
    #[serde(default)]
    exclude_regex_any: Vec<String>,
}

impl CheckConfig {
    /// Generate detection prompt by substituting function source
    fn format_detection_prompt(&self, func: &FunctionInfo) -> String {
        let mut prompt = self.detection_prompt
            .replace("{function_source}", &func.source_no_docstring)
            .replace("{name}", &self.name)
            .replace("{keyword}", &self.keyword);

        // Add special context for __init__ methods to reduce false positives
        if func.name == "__init__" {
            let context = "\n\nIMPORTANT: This is an __init__ (constructor) method that initializes object state. \
                          Constructor methods typically run once per object and should NOT be flagged unless they \
                          have genuine algorithmic complexity issues (like nested loops over input data). \
                          Simple attribute assignments and one-time setup calls are NOT performance issues.\n";

            // Insert the context before the final assistant prompt marker
            if let Some(pos) = prompt.rfind("<|im_start|>assistant") {
                prompt.insert_str(pos, context);
            } else {
                // Fallback: append at the end
                prompt.push_str(context);
            }
        }

        prompt
    }

    /// Generate solution prompt by substituting function source
    fn format_solution_prompt(&self, func: &FunctionInfo) -> String {
        self.solution_prompt
            .replace("{function_source}", &func.source_no_docstring)
            .replace("{keyword}", &self.keyword)
    }

    /// Generate verifier prompt by substituting function source and solution
    fn format_verifier_prompt(&self, func: &FunctionInfo, solution: &str) -> String {
        self.verifier_prompt
            .replace("{function_source}", &func.source_no_docstring)
            .replace("{solution}", solution)
            .replace("{keyword}", &self.keyword)
    }

    /// Parse structured detection output
    /// Expected format: VERDICT: OK|{keyword}, CONFIDENCE: 0.0-1.0, DETAIL: text, END
    fn parse_detection(&self, response: &str) -> ParsedDetection {
        let mut has_issue = false;
        let mut confidence: Option<f32> = None;
        let mut detail = String::new();

        for line in response.lines() {
            let trimmed = line.trim();

            if trimmed.starts_with("VERDICT:") {
                let verdict = trimmed[8..].trim().to_uppercase();
                has_issue = verdict == self.keyword.to_uppercase();
            } else if trimmed.starts_with("CONFIDENCE:") {
                if let Ok(val) = trimmed[11..].trim().parse::<f32>() {
                    confidence = Some(val.clamp(0.0, 1.0));
                }
            } else if trimmed.starts_with("DETAIL:") {
                detail = trimmed[7..].trim().to_string();
            } else if trimmed == "END" {
                break;
            }
        }

        ParsedDetection { has_issue, confidence, _detail: detail }
    }
}

/// Parse verifier output
fn parse_verification_result(response: &str) -> VerificationResult {
    let mut is_valid = false;
    let mut reason = String::new();

    for line in response.lines() {
        let trimmed = line.trim();

        if trimmed.starts_with("VERDICT:") {
            let verdict = trimmed[8..].trim().to_uppercase();
            is_valid = verdict == "VALID";
        } else if trimmed.starts_with("REASON:") {
            reason = trimmed[7..].trim().to_string();
        } else if trimmed == "END" {
            break;
        }
    }

    VerificationResult { is_valid, reason }
}

/// Get the default built-in checks configuration as a TOML string
fn get_default_config_toml() -> &'static str {
    include_str!("../loopsleuth.toml")
}

/// Apply config settings to CLI arguments (CLI takes precedence)
fn apply_config_settings(cli: &mut Cli, config: &ChecksConfig) {
    let settings = &config.settings;

    // Only apply config settings if CLI argument wasn't provided
    if cli.model.is_none() {
        cli.model = settings.model.clone();
    }
    if cli.threads == 4 && settings.threads.is_some() {
        // 4 is the default, so override with config if present
        cli.threads = settings.threads.unwrap();
    }
    if cli.max_tokens == 1024 && settings.max_tokens.is_some() {
        // 1024 is the default, so override with config if present
        cli.max_tokens = settings.max_tokens.unwrap();
    }
    if cli.context_size == 4096 && settings.context_size.is_some() {
        // 4096 is the default, so override with config if present
        cli.context_size = settings.context_size.unwrap();
    }
    if cli.skip_large == 0 && settings.skip_large.is_some() {
        // 0 is the default, so override with config if present
        cli.skip_large = settings.skip_large.unwrap();
    }
    if cli.cache_dir.is_none() {
        cli.cache_dir = settings.cache_dir.clone();
    }
}

/// Load checks configuration from file or use defaults
fn load_checks_config(config_path: Option<PathBuf>) -> Result<ChecksConfig> {
    // If specific config file provided, load it
    if let Some(path) = config_path {
        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        let mut config: ChecksConfig = toml::from_str(&content)
            .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
        apply_template_expansion(&mut config)
            .with_context(|| format!("Failed to expand templates in config file: {}", path.display()))?;
        return Ok(config);
    }

    // Try ~/.config/loopsleuth/loopsleuth.toml
    if let Some(home) = std::env::var_os("HOME") {
        let config_path = PathBuf::from(home)
            .join(".config")
            .join("loopsleuth")
            .join("loopsleuth.toml");

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .with_context(|| format!("Failed to read config file: {}", config_path.display()))?;
            let mut config: ChecksConfig = toml::from_str(&content)
                .with_context(|| format!("Failed to parse config file: {}", config_path.display()))?;
            apply_template_expansion(&mut config)
                .with_context(|| format!("Failed to expand templates in config file: {}", config_path.display()))?;
            return Ok(config);
        }
    }

    // Fall back to built-in defaults
    let mut config: ChecksConfig = toml::from_str(get_default_config_toml())
        .context("Failed to parse built-in default configuration")?;
    apply_template_expansion(&mut config)
        .context("Failed to expand templates in built-in default configuration")?;
    Ok(config)
}

// CheckConfig removed - now using CheckConfig directly from loaded configuration

/// Registry of all available checks - loaded from configuration
fn get_all_checks(cli: &Cli) -> Result<Vec<CheckConfig>> {
    let config = load_checks_config(cli.config.clone())?;
    Ok(config.check)
}

/// Expand {template:name} placeholders and inject detection/fix blocks.
fn apply_template_expansion(config: &mut ChecksConfig) -> Result<()> {
    let templates = &config.templates;

    for check in &mut config.check {
        warn_missing_template_refs(check, templates);
        validate_guard_patterns(check)
            .with_context(|| format!("Failed to validate guard patterns for check '{}'", check.key))?;

        check.detection_prompt = expand_template_string(&check.detection_prompt, templates)
            .context("Failed to expand detection prompt template")?;
        check.detection_prompt = check
            .detection_prompt
            .replace("{detection_rules}", &check.detection_rules);

        check.solution_prompt = expand_template_string(&check.solution_prompt, templates)
            .context("Failed to expand solution prompt template")?;
        check.solution_prompt = check
            .solution_prompt
            .replace("{fix_recipes}", &check.fix_recipes);

        check.verifier_prompt = expand_template_string(&check.verifier_prompt, templates)
            .context("Failed to expand verifier prompt template")?;
        check.verifier_prompt = check
            .verifier_prompt
            .replace("{detection_rules}", &check.detection_rules)
            .replace("{fix_recipes}", &check.fix_recipes);
    }

    Ok(())
}

fn validate_guard_patterns(check: &CheckConfig) -> Result<()> {
    for pattern in check.guard.require_regex_any.iter()
        .chain(check.guard.require_regex_all.iter())
        .chain(check.guard.exclude_regex_any.iter())
    {
        Regex::new(pattern)
            .with_context(|| format!("Invalid regex pattern: {}", pattern))?;
    }
    Ok(())
}

fn guard_skip_reason(check: &CheckConfig, func: &FunctionInfo) -> Result<Option<String>> {
    let text = &func.source_no_docstring;

    if !check.guard.require_any.is_empty()
        && !check.guard.require_any.iter().any(|t| text.contains(t))
    {
        return Ok(Some("guard require_any missing".to_string()));
    }

    if !check.guard.require_all.is_empty()
        && !check.guard.require_all.iter().all(|t| text.contains(t))
    {
        return Ok(Some("guard require_all missing".to_string()));
    }

    if !check.guard.exclude_any.is_empty()
        && check.guard.exclude_any.iter().any(|t| text.contains(t))
    {
        return Ok(Some("guard exclude_any hit".to_string()));
    }

    if !check.guard.require_regex_any.is_empty() {
        let mut matched = false;
        for pattern in &check.guard.require_regex_any {
            let re = Regex::new(pattern)?;
            if re.is_match(text) {
                matched = true;
                break;
            }
        }
        if !matched {
            return Ok(Some("guard require_regex_any missing".to_string()));
        }
    }

    if !check.guard.require_regex_all.is_empty() {
        for pattern in &check.guard.require_regex_all {
            let re = Regex::new(pattern)?;
            if !re.is_match(text) {
                return Ok(Some("guard require_regex_all missing".to_string()));
            }
        }
    }

    if !check.guard.exclude_regex_any.is_empty() {
        for pattern in &check.guard.exclude_regex_any {
            let re = Regex::new(pattern)?;
            if re.is_match(text) {
                return Ok(Some("guard exclude_regex_any hit".to_string()));
            }
        }
    }

    Ok(None)
}

fn warn_missing_template_refs(
    check: &CheckConfig,
    templates: &std::collections::HashMap<String, String>,
) {
    let mut missing: Vec<String> = Vec::new();
    for prompt in [&check.detection_prompt, &check.solution_prompt, &check.verifier_prompt] {
        if let Some(name) = get_template_name(prompt) {
            if !templates.contains_key(name) && !missing.contains(&name.to_string()) {
                missing.push(name.to_string());
            }
        }
    }

    if !missing.is_empty() {
        eprintln!(
            "Warning: check '{}' references missing templates: {}",
            check.key,
            missing.join(", ")
        );
    }
}

fn expand_template_string(
    prompt: &str,
    templates: &std::collections::HashMap<String, String>,
) -> Result<String> {
    let Some(name) = get_template_name(prompt) else {
        return Ok(prompt.to_string());
    };

    templates
        .get(name)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Unknown template: {}", name))
}

fn get_template_name(prompt: &str) -> Option<&str> {
    let trimmed = prompt.trim();
    trimmed
        .strip_prefix("{template:")
        .and_then(|s| s.strip_suffix('}'))
}


/// Parse check keys from comma-separated string
fn parse_check_keys(keys: &str) -> Vec<String> {
    keys.split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Get the checks to run based on CLI arguments
fn get_checks_to_run(cli: &Cli) -> Result<Vec<CheckConfig>> {
    let all_checks = get_all_checks(cli)?;

    // If specific checks requested, filter to those
    if let Some(check_list) = &cli.checks {
        let requested_keys = parse_check_keys(check_list);
        return Ok(all_checks
            .into_iter()
            .filter(|check| requested_keys.contains(&check.key))
            .collect());
    }

    // If excludes specified, filter those out
    if let Some(exclude_list) = &cli.exclude {
        let excluded_keys = parse_check_keys(exclude_list);
        return Ok(all_checks
            .into_iter()
            .filter(|check| !excluded_keys.contains(&check.key))
            .collect());
    }

    // Default: run all checks
    Ok(all_checks)
}

/// List all available checks
fn list_all_checks(cli: &Cli) -> Result<()> {
    let checks = get_all_checks(cli)?;

    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║                   AVAILABLE CHECKS                            ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!();

    let mut by_category: std::collections::HashMap<&str, Vec<&CheckConfig>> = std::collections::HashMap::new();
    for check in &checks {
        by_category.entry(&check.category).or_insert_with(Vec::new).push(check);
    }

    for (category, checks) in by_category.iter() {
        println!("📂 {}:", category.to_uppercase());
        println!();
        for check in checks {
            println!("  {} ({})", check.name, check.key);
            println!("    {}", check.description);
            println!();
        }
    }

    println!("Usage examples:");
    println!("  # Run all checks (default)");
    println!("  loopsleuth -m model.gguf ./src");
    println!();
    println!("  # Run specific checks only");
    println!("  loopsleuth -m model.gguf ./src --checks quadratic,linear-in-loop");
    println!();
    println!("  # Run all except ML-specific checks");
    println!("  loopsleuth -m model.gguf ./src --exclude conversion-churn,ml-footguns");
    println!();

    Ok(())
}

#[derive(Clone)]
struct CheckResult {
    check_key: String,
    check_name: String,
    has_issue: bool,
    analysis: String,
    solution: Option<String>,
}

fn dedupe_check_results(mut results: Vec<CheckResult>, rules: &[DedupeRule]) -> Vec<CheckResult> {
    for rule in rules {
        if rule.prefer.is_empty() || rule.drop.is_empty() {
            continue;
        }
        let has_prefer = results
            .iter()
            .any(|r| r.has_issue && r.check_key == rule.prefer);
        if has_prefer {
            results.retain(|r| !(r.has_issue && rule.drop.contains(&r.check_key)));
        }
    }
    results
}

#[derive(Clone)]
struct AnalysisResult {
    function: FunctionInfo,
    check_results: Vec<CheckResult>,
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
    has_issue: bool,
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

        // Migrate old schema if it exists
        Self::migrate_schema(&conn)?;

        // Create new table if it doesn't exist
        conn.execute(
            "CREATE TABLE IF NOT EXISTS check_results (
                function_hash TEXT NOT NULL,
                check_key TEXT NOT NULL,
                has_issue INTEGER NOT NULL,
                analysis TEXT NOT NULL,
                solution TEXT,
                created_at INTEGER NOT NULL,
                PRIMARY KEY (function_hash, check_key)
            )",
            [],
        )?;

        Ok(Self {
            conn,
            enabled: true,
        })
    }

    /// Migrate from old schema to new schema
    fn migrate_schema(conn: &Connection) -> Result<()> {
        // Check if old table exists
        let old_table_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='analysis_results'",
                [],
                |row| row.get::<_, i32>(0).map(|count| count > 0),
            )?;

        if !old_table_exists {
            return Ok(());
        }

        // Check if new table exists
        let new_table_exists: bool = conn
            .query_row(
                "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='check_results'",
                [],
                |row| row.get::<_, i32>(0).map(|count| count > 0),
            )?;

        if new_table_exists {
            // Already migrated
            return Ok(());
        }

        // Create new table
        conn.execute(
            "CREATE TABLE check_results (
                function_hash TEXT NOT NULL,
                check_key TEXT NOT NULL,
                has_issue INTEGER NOT NULL,
                analysis TEXT NOT NULL,
                solution TEXT,
                created_at INTEGER NOT NULL,
                PRIMARY KEY (function_hash, check_key)
            )",
            [],
        )?;

        // Migrate data from old table to new table with check_key='quadratic'
        conn.execute(
            "INSERT INTO check_results (function_hash, check_key, has_issue, analysis, solution, created_at)
             SELECT function_hash, 'quadratic', is_quadratic, analysis, solution, created_at
             FROM analysis_results",
            [],
        )?;

        // Drop old table
        conn.execute("DROP TABLE analysis_results", [])?;

        Ok(())
    }

    /// Compute SHA256 hash of function source code
    fn hash_function(source: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(source.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Check if analysis result exists in cache
    fn get(&self, func: &FunctionInfo, check_key: &str) -> Result<Option<CachedResult>> {
        if !self.enabled {
            return Ok(None);
        }

        let hash = Self::hash_function(&func.source);

        let mut stmt = self.conn.prepare(
            "SELECT has_issue, analysis, solution FROM check_results WHERE function_hash = ?1 AND check_key = ?2"
        )?;

        let result = stmt.query_row(params![hash, check_key], |row| {
            Ok(CachedResult {
                has_issue: row.get::<_, i32>(0)? != 0,
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
    fn put(&self, func: &FunctionInfo, check_key: &str, has_issue: bool, analysis: &str, solution: Option<&str>) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        let hash = Self::hash_function(&func.source);
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs() as i64;

        self.conn.execute(
            "INSERT OR REPLACE INTO check_results (function_hash, check_key, has_issue, analysis, solution, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![hash, check_key, has_issue as i32, analysis, solution, timestamp],
        )?;

        Ok(())
    }

    /// Clear all cache entries
    fn clear(&self) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        self.conn.execute("DELETE FROM check_results", [])?;
        Ok(())
    }

    /// Get cache statistics
    fn stats(&self) -> Result<(usize, usize)> {
        if !self.enabled {
            return Ok((0, 0));
        }

        let total: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM check_results",
            [],
            |row| row.get(0)
        )?;

        let with_issues: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM check_results WHERE has_issue = 1",
            [],
            |row| row.get(0)
        )?;

        Ok((total, with_issues))
    }
}

/// RAII guard that redirects stderr to /dev/null and restores it on drop (Unix only)
#[cfg(unix)]
struct StderrSuppressor {
    original_stderr: Option<i32>,
}

#[cfg(unix)]
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

#[cfg(unix)]
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

/// No-op stderr suppressor for Windows (stderr suppression not available)
#[cfg(not(unix))]
struct StderrSuppressor;

#[cfg(not(unix))]
impl StderrSuppressor {
    fn new() -> Result<Self> {
        Ok(StderrSuppressor)
    }
}

/// RAII guard that redirects stdout to /dev/null and restores it on drop (Unix only)
#[cfg(unix)]
struct StdoutSuppressor {
    original_stdout: Option<i32>,
}

#[cfg(unix)]
impl StdoutSuppressor {
    fn new() -> Result<Self> {
        unsafe {
            // Duplicate stdout so we can restore it later
            let original_stdout = libc::dup(libc::STDOUT_FILENO);
            if original_stdout < 0 {
                return Err(anyhow::anyhow!("Failed to duplicate stdout"));
            }

            // Open /dev/null
            let devnull = OpenOptions::new()
                .write(true)
                .open("/dev/null")?;

            // Redirect stdout to /dev/null
            if libc::dup2(devnull.as_raw_fd(), libc::STDOUT_FILENO) < 0 {
                return Err(anyhow::anyhow!("Failed to redirect stdout"));
            }

            Ok(StdoutSuppressor {
                original_stdout: Some(original_stdout),
            })
        }
    }
}

#[cfg(unix)]
impl Drop for StdoutSuppressor {
    fn drop(&mut self) {
        if let Some(original) = self.original_stdout {
            unsafe {
                libc::dup2(original, libc::STDOUT_FILENO);
                libc::close(original);
            }
        }
    }
}

/// No-op stdout suppressor for Windows (stdout suppression not available)
#[cfg(not(unix))]
struct StdoutSuppressor;

#[cfg(not(unix))]
impl StdoutSuppressor {
    fn new() -> Result<Self> {
        Ok(StdoutSuppressor)
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

    let mut cli = Cli::parse();

    // Handle --list-checks flag
    if cli.list_checks {
        list_all_checks(&cli)?;
        return Ok(());
    }

    // Handle --print-default-config flag
    if cli.print_default_config {
        print!("{}", get_default_config_toml());
        return Ok(());
    }

    // Load config and apply default settings (CLI args take precedence)
    let config = load_checks_config(cli.config.clone())?;
    apply_config_settings(&mut cli, &config);

    // Validate required arguments
    let python_path = cli.python_path.as_ref()
        .ok_or_else(|| anyhow::anyhow!("PATH argument is required (unless using --list-checks)"))?;
    let model_path = cli.model.as_ref()
        .ok_or_else(|| anyhow::anyhow!("--model argument is required (unless using --list-checks or providing model in config)"))?;

    // Get checks to run
    let checks = get_checks_to_run(&cli)?;
    if checks.is_empty() {
        return Err(anyhow::anyhow!("No checks selected. Use --checks to specify checks or --list-checks to see available checks."));
    }

    // Show initialization message (before suppressor)
    println!("🔧 Initializing LoopSleuth...");

    // Suppress llama.cpp logs unless verbose mode is enabled
    // Keep the suppressor active for the entire run
    let _suppressor = if !cli.verbose {
        println!("   ⚙️  Setting up LLM backend...");
        println!("   📦 Loading model: {}...", model_path.display());
        Some(StderrSuppressor::new()?)
    } else {
        None
    };

    // Initialize llama backend
    let backend = LlamaBackend::init()?;

    // Load model
    let model_params = LlamaModelParams::default();
    let model = LlamaModel::load_from_file(&backend, model_path, &model_params)
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
    let python_files = collect_python_files(python_path)?;
    let file_count = python_files.len();

    println!("🔍 Scanning {} Python file(s)...", file_count);
    println!("🔬 Running {} check(s): {}",
        checks.len(),
        checks.iter().map(|c| c.key.clone()).collect::<Vec<_>>().join(", ")
    );

    // First pass: count total functions
    let mut total_functions_count = 0;
    for path in &python_files {
        if let Ok(mut functions) = extract_functions(path) {
            // Apply function name filter if specified
            if let Some(ref filter) = cli.filter_function {
                let filter_lower = filter.to_lowercase();
                functions.retain(|func| func.name.to_lowercase().contains(&filter_lower));
            }
            total_functions_count += functions.len();
        }
    }

    if let Some(ref filter) = cli.filter_function {
        println!("🔍 Filtering functions matching: \"{}\"", filter);
    }
    println!("📊 Analyzing {} function(s)...\n", total_functions_count);

    let mut all_file_results: Vec<FileResults> = Vec::new();
    let mut total_functions = 0;
    let mut current_func_num = 0;
    let mut functions_with_issues = 0;
    let mut total_stats = TokenStats::default();

    // Process each file
    for file_path in &python_files {
        let mut functions = extract_functions(&file_path)?;

        // Apply function name filter if specified
        if let Some(ref filter) = cli.filter_function {
            let filter_lower = filter.to_lowercase();
            functions.retain(|func| func.name.to_lowercase().contains(&filter_lower));
        }

        let mut file_results = Vec::new();

        for func in functions {
            total_functions += 1;
            current_func_num += 1;

            // Calculate progress bar
            let progress_pct = (current_func_num as f32 / total_functions_count as f32 * 100.0) as usize;
            let bar_width = 30;
            let filled = (current_func_num as f32 / total_functions_count as f32 * bar_width as f32) as usize;
            let empty = bar_width - filled;
            let progress_bar = format!("[{}{}]", "█".repeat(filled), "░".repeat(empty));

            // Get filename for display
            let filename = file_path.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown");

            // Include class name in display if function is a method
            let func_display = if let Some(ref class_name) = func.class_name {
                format!("{}::{}::{}", filename, class_name, func.name)
            } else {
                format!("{}::{}", filename, func.name)
            };

            // Skip large functions if requested
            if cli.skip_large > 0 {
                let line_count = func.source.lines().count();
                if line_count > cli.skip_large {
                    print!("\r\x1b[K{} {}% [{}/{}] | Issues: {} | ⊗ Skipped: {} (too large)",
                           progress_bar, progress_pct, current_func_num, total_functions_count,
                           functions_with_issues, func_display);
                    std::io::Write::flush(&mut std::io::stdout()).ok();
                    continue;
                }
            }

            // Run all checks for this function
            let mut check_results = Vec::new();

            for check in &checks {
                if let Some(reason) = guard_skip_reason(check, &func)? {
                    let analysis = format!(
                        "VERDICT: OK\nCONFIDENCE: 0.00\nDETAIL: Skipped by guard ({})\nEND",
                        reason
                    );
                    let _ = cache.put(&func, &check.key, false, &analysis, None);
                    check_results.push(CheckResult {
                        check_key: check.key.to_string(),
                        check_name: check.name.to_string(),
                        has_issue: false,
                        analysis,
                        solution: None,
                    });
                    print!("\r\x1b[K{} {}% [{}/{}] | Issues: {} | ⏭️  [{}] {}",
                           progress_bar, progress_pct, current_func_num, total_functions_count,
                           functions_with_issues, check.key, func_display);
                    std::io::Write::flush(&mut std::io::stdout()).ok();
                    continue;
                }

                // Check cache first
                if let Ok(Some(cached)) = cache.get(&func, &check.key) {
                    // Cache hit
                    check_results.push(CheckResult {
                        check_key: check.key.to_string(),
                        check_name: check.name.to_string(),
                        has_issue: cached.has_issue,
                        analysis: cached.analysis,
                        solution: cached.solution,
                    });

                    print!("\r\x1b[K{} {}% [{}/{}] | Issues: {} | 💾 [{}] {}",
                           progress_bar, progress_pct, current_func_num, total_functions_count,
                           functions_with_issues, check.key, func_display);
                    std::io::Write::flush(&mut std::io::stdout()).ok();

                    continue;
                }

                // Cache miss - run LLM detection
                print!("\r\x1b[K{} {}% [{}/{}] | Issues: {} | 🔍 [{}] {}",
                       progress_bar, progress_pct, current_func_num, total_functions_count,
                       functions_with_issues, check.key, func_display);
                std::io::Write::flush(&mut std::io::stdout()).ok();

                // Run detection
                let detection_prompt = check.format_detection_prompt(&func);
                let detection_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    generate_response(&model, &mut ctx, &detection_prompt, cli.max_tokens, cli.verbose)
                }));

                let detection_result = match detection_result {
                    Ok(res) => res,
                    Err(_) => {
                        print!("\r\x1b[K{} {}% [{}/{}] | Issues: {} | 💥 [{}] Error",
                               progress_bar, progress_pct, current_func_num, total_functions_count,
                               functions_with_issues, check.key);
                        std::io::Write::flush(&mut std::io::stdout()).ok();
                        continue;
                    }
                };

                match detection_result {
                    Ok((analysis, _truncated, stats)) => {
                        total_stats.add(&stats);
                        let detection = check.parse_detection(&analysis);
                        let has_issue = detection.has_issue;

                        // Store confidence in analysis for debugging
                        let enhanced_analysis = if let Some(conf) = detection.confidence {
                            format!("{}\n[Confidence: {:.2}]", analysis, conf)
                        } else {
                            analysis.clone()
                        };

                        if has_issue {
                            // Generate solution
                            print!("\r\x1b[K{} {}% [{}/{}] | Issues: {} | 💡 [{}] Solution...",
                                   progress_bar, progress_pct, current_func_num, total_functions_count,
                                   functions_with_issues, check.key);
                            std::io::Write::flush(&mut std::io::stdout()).ok();

                            let solution_prompt = check.format_solution_prompt(&func);
                            let solution_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                generate_response(&model, &mut ctx, &solution_prompt, cli.max_tokens, cli.verbose)
                            }))
                            .ok()
                            .and_then(|r| r.ok());

                            let solution_text = solution_result.as_ref().map(|(text, _truncated, _stats)| text.clone());

                            // Accumulate stats from solution generation
                            if let Some((_text, _truncated, stats)) = solution_result {
                                total_stats.add(&stats);
                            }

                            // Extract optimized function and generate diff
                            let optimized_and_diff = solution_text.as_ref()
                                .and_then(|sol| {
                                    let optimized = extract_optimized_function(sol)?;

                                    if let Err(reason) = validate_optimization(&func.source_no_docstring, &optimized) {
                                        return Some(Err(reason));
                                    }

                                    let diff = generate_diff(&func.source_no_docstring, &optimized);
                                    Some(Ok((optimized, diff)))
                                });

                            let (_optimized_code, diff) = match optimized_and_diff {
                                Some(Ok(pair)) => pair,
                                Some(Err(reason)) => {
                                    let failure_note = format!(
                                        "{}\n\n[No safe change suggested: {}]",
                                        enhanced_analysis, reason
                                    );
                                    if cli.verbose {
                                        eprintln!(
                                            "Verifier/validation: rejected solution for {} ({}): {}",
                                            check.key,
                                            func.name,
                                            reason
                                        );
                                    }
                                    let _ = cache.put(&func, &check.key, true, &failure_note, None);

                                    check_results.push(CheckResult {
                                        check_key: check.key.to_string(),
                                        check_name: check.name.to_string(),
                                        has_issue: true,
                                        analysis: failure_note,
                                        solution: None,
                                    });

                                    continue;
                                }
                                None => {
                                    let failure_note = format!(
                                        "{}\n\n[No safe change suggested: Could not extract optimized function]",
                                        enhanced_analysis
                                    );
                                    if cli.verbose {
                                        eprintln!(
                                            "Verifier/validation: rejected solution for {} ({}): could not extract optimized function",
                                            check.key,
                                            func.name
                                        );
                                    }
                                    let _ = cache.put(&func, &check.key, true, &failure_note, None);

                                    check_results.push(CheckResult {
                                        check_key: check.key.to_string(),
                                        check_name: check.name.to_string(),
                                        has_issue: true,
                                        analysis: failure_note,
                                        solution: None,
                                    });

                                    continue;
                                }
                            };
                            let solution = Some(format!("```diff\n{}\n```", diff));

                            // Diff is valid according to validate_diff, run verifier if available
                            if !check.verifier_prompt.is_empty() {
                                print!("\r\x1b[K{} {}% [{}/{}] | Issues: {} | 🔍 [{}] Verifying solution...",
                                       progress_bar, progress_pct, current_func_num, total_functions_count,
                                       functions_with_issues, check.key);
                                std::io::Write::flush(&mut std::io::stdout()).ok();

                                let verifier_prompt = check.format_verifier_prompt(&func, solution.as_ref().unwrap());
                                let verifier_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                                    generate_response(&model, &mut ctx, &verifier_prompt, cli.max_tokens, cli.verbose)
                                }))
                                .ok()
                                .and_then(|r| r.ok());

                                if let Some((verifier_output, _truncated, stats)) = verifier_result {
                                    total_stats.add(&stats);
                                    let verification = parse_verification_result(&verifier_output);

                                    if !verification.is_valid {
                                        // Verifier rejected - store detection but no solution
                                        let rejection_note = format!("{}\n\n[Verifier rejected: {}]",
                                                                    enhanced_analysis, verification.reason);
                                        if cli.verbose {
                                            eprintln!(
                                                "Verifier rejected solution for {} ({}): {}",
                                                check.key,
                                                func.name,
                                                verification.reason
                                            );
                                        }
                                        let _ = cache.put(&func, &check.key, true, &rejection_note, None);

                                        check_results.push(CheckResult {
                                            check_key: check.key.to_string(),
                                            check_name: check.name.to_string(),
                                            has_issue: true,
                                            analysis: rejection_note,
                                            solution: None,
                                        });

                                                continue;
                                    }
                                }
                            }

                            // Verifier passed (or no verifier) - store in cache
                            let _ = cache.put(&func, &check.key, true, &enhanced_analysis, solution.as_deref());

                            check_results.push(CheckResult {
                                check_key: check.key.to_string(),
                                check_name: check.name.to_string(),
                                has_issue: true,
                                analysis: enhanced_analysis,
                                solution,
                            });
                        } else {
                            // No issue - store in cache
                            let _ = cache.put(&func, &check.key, false, &enhanced_analysis, None);

                            check_results.push(CheckResult {
                                check_key: check.key.to_string(),
                                check_name: check.name.to_string(),
                                has_issue: false,
                                analysis: enhanced_analysis,
                                solution: None,
                            });
                        }
                    }
                    Err(e) => {
                        let error_msg = e.to_string();
                        print!("\r\x1b[K{} {}% [{}/{}] | Issues: {} | ⚠️  [{}] {}",
                               progress_bar, progress_pct, current_func_num, total_functions_count,
                               functions_with_issues, check.key,
                               if error_msg.contains("too large") { "Too large" } else { "Error" });
                        std::io::Write::flush(&mut std::io::stdout()).ok();
                        // Log the actual error for debugging (use println to avoid stderr suppression)
                        println!("\n   Debug: Error in {}: {}", func.name, error_msg);
                    }
                }
            }

            // Only count as having issues if we actually added results with issues
            // (not just detected issues that were later rejected due to invalid diffs)
            let check_results = dedupe_check_results(check_results, &config.dedupe);
            let actually_has_issues = check_results.iter().any(|r| r.has_issue);
            if actually_has_issues {
                functions_with_issues += 1;
            }

            if !check_results.is_empty() {
                file_results.push(AnalysisResult {
                    function: func,
                    check_results,
                });
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

    // Flatten results for summary
    let all_results: Vec<AnalysisResult> = all_file_results
        .iter()
        .flat_map(|fr| fr.results.iter())
        .cloned()
        .collect();

    // Print concise summary
    print_summary(&all_file_results, file_count, total_functions, functions_with_issues, &checks, &cache, cli.no_cache, &total_stats);

    // Print detailed markdown report only if --details flag is set
    if functions_with_issues > 0 && cli.details {
        print_detailed_report(&all_results);
    } else if functions_with_issues > 0 && !cli.details && cli.output.is_none() {
        println!("💡 Tip: Use --details to see full analysis or --output FILE to save report");
        println!();
    }

    // Save to file if requested (always includes full details)
    if let Some(output_path) = &cli.output {
        write_report_to_file(output_path, &all_results, total_functions, functions_with_issues, &checks, &cache, cli.no_cache)?;
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
        extract_functions_from_body(&module.body, &source, file_path, None, &mut functions);
    }

    Ok(functions)
}

fn extract_functions_from_body(
    body: &[Stmt],
    source: &str,
    file_path: &PathBuf,
    class_name: Option<String>,
    functions: &mut Vec<FunctionInfo>,
) {
    for stmt in body {
        match stmt {
            Stmt::FunctionDef(func_def) => {
                let func_source = extract_source_from_range(&source, func_def.range.start(), func_def.range.end());
                let line_number = count_lines_to_offset(&source, func_def.range.start());
                let func_source_no_docstring = strip_docstring(&func_source);

                functions.push(FunctionInfo {
                    name: func_def.name.to_string(),
                    source: func_source,
                    source_no_docstring: func_source_no_docstring,
                    file_path: file_path.clone(),
                    line_number,
                    class_name: class_name.clone(),
                });
            }
            Stmt::AsyncFunctionDef(func_def) => {
                let func_source = extract_source_from_range(&source, func_def.range.start(), func_def.range.end());
                let line_number = count_lines_to_offset(&source, func_def.range.start());
                let func_source_no_docstring = strip_docstring(&func_source);

                functions.push(FunctionInfo {
                    name: func_def.name.to_string(),
                    source: func_source,
                    source_no_docstring: func_source_no_docstring,
                    file_path: file_path.clone(),
                    line_number,
                    class_name: class_name.clone(),
                });
            }
            Stmt::ClassDef(class_def) => {
                // Recursively extract functions from class bodies
                extract_functions_from_body(
                    &class_def.body,
                    source,
                    file_path,
                    Some(class_def.name.to_string()),
                    functions
                );
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

/// Strip docstrings from Python function source to reduce token usage
fn strip_docstring(source: &str) -> String {
    let lines: Vec<&str> = source.lines().collect();
    if lines.is_empty() {
        return source.to_string();
    }

    let mut result = Vec::new();
    let mut i = 0;
    let mut found_def = false;

    while i < lines.len() {
        let line = lines[i];
        let trimmed = line.trim();

        // Keep the def line
        if trimmed.starts_with("def ") || trimmed.starts_with("async def ") {
            result.push(line);
            found_def = true;
            i += 1;
            continue;
        }

        // After def, look for docstring
        if found_def && (trimmed.starts_with("\"\"\"") || trimmed.starts_with("'''")) {
            let quote = if trimmed.starts_with("\"\"\"") { "\"\"\"" } else { "'''" };

            // Check if it's a single-line docstring
            if trimmed.ends_with(quote) && trimmed.len() > 6 {
                // Single-line docstring - skip it
                i += 1;
                found_def = false;  // Only strip first docstring after def
                continue;
            }

            // Multi-line docstring - skip until closing quotes
            i += 1;
            while i < lines.len() {
                if lines[i].trim().ends_with(quote) {
                    i += 1;
                    break;
                }
                i += 1;
            }
            found_def = false;  // Only strip first docstring after def
            continue;
        }

        // Keep all other lines
        result.push(line);
        found_def = false;  // Reset after any non-docstring line
        i += 1;
    }

    result.join("\n")
}

/// Extract confidence percentage from analysis text
/// Looks for "[Confidence: X.XX]" pattern and converts to percentage
fn extract_confidence_percentage(analysis: &str) -> u32 {
    // Look for [Confidence: X.XX] pattern
    if let Some(start) = analysis.find("[Confidence: ") {
        if let Some(end) = analysis[start..].find(']') {
            let conf_str = &analysis[start + 13..start + end];
            if let Ok(conf_float) = conf_str.parse::<f32>() {
                return (conf_float * 100.0).round() as u32;
            }
        }
    }
    // Default to 0 if not found
    0
}

fn generate_response(
    model: &LlamaModel,
    ctx: &mut LlamaContext,
    prompt: &str,
    max_tokens: i32,
    verbose: bool,
) -> Result<(String, bool, TokenStats)> {  // Returns (response, was_truncated, token_stats)
    // Start timing
    let start_time = Instant::now();

    // Show prompt in verbose mode
    if verbose {
        println!("\n╔════════════════════════════════════════════════════════════════");
        println!("║ PROMPT");
        println!("╚════════════════════════════════════════════════════════════════");
        println!("{}", prompt);
        println!("────────────────────────────────────────────────────────────────\n");
    }

    // Suppress any backend stdout noise during generation in verbose mode
    let (stdout_suppressor, stderr_suppressor) = if verbose {
        (Some(StdoutSuppressor::new()?), Some(StderrSuppressor::new()?))
    } else {
        (None, None)
    };

    // Tokenize the prompt (AddBos::Always adds BOS token)
    let tokens = model.str_to_token(prompt, llama_cpp_2::model::AddBos::Always)?;
    let input_token_count = tokens.len();

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
    let mut hit_eog = false;
    let mut output_token_count = 0;

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
            hit_eog = true;
            break;
        }

        // Convert token to string
        // Handle UTF-8 conversion errors gracefully (some tokens may be incomplete multi-byte sequences)
        match model.token_to_str(new_token, llama_cpp_2::model::Special::Tokenize) {
            Ok(token_str) => {
                response.push_str(&token_str);
                output_token_count += 1;
            }
            Err(_) => {
                // Skip tokens that can't be converted to valid UTF-8
                // This can happen with incomplete multi-byte UTF-8 sequences in some models
                // Continue generating to see if subsequent tokens form valid sequences
                continue;
            }
        }

        // Check for custom stop sequence "END" on its own line
        if let Some(last_line) = response.lines().last() {
            if last_line.trim() == "END" {
                // Remove the END line from response
                if let Some(pos) = response.rfind('\n') {
                    response.truncate(pos);
                }
                hit_eog = true;
                break;
            }
        }

        // Prepare next batch
        batch.clear();
        batch.add(new_token, n_cur, &[0], true)?;
        ctx.decode(&mut batch)?;

        n_cur += 1;
    }

    let generation_time = start_time.elapsed();
    drop(stdout_suppressor);
    drop(stderr_suppressor);
    let was_truncated = !hit_eog;

    // If truncated, clean up any unclosed markdown code blocks
    let cleaned_response = if was_truncated {
        fix_truncated_markdown(&response)
    } else {
        response
    };

    let stats = TokenStats::new(input_token_count, output_token_count, generation_time);

    // Show response in verbose mode
    if verbose {
        println!("\n╔════════════════════════════════════════════════════════════════");
        println!("║ RESPONSE");
        println!("╚════════════════════════════════════════════════════════════════");
        println!("{}", cleaned_response);
        println!("────────────────────────────────────────────────────────────────");
        println!("📊 Tokens: {} in, {} out | Speed: {:.1} tok/s | Time: {:.1}s{}",
            input_token_count, output_token_count,
            stats.tokens_per_second(), generation_time.as_secs_f64(),
            if was_truncated { " | ⚠️ TRUNCATED" } else { "" }
        );
        println!("────────────────────────────────────────────────────────────────\n");
    }

    Ok((cleaned_response, was_truncated, stats))
}

/// Fix truncated markdown by closing unclosed code blocks and adding truncation notice
fn fix_truncated_markdown(text: &str) -> String {
    let mut result = text.to_string();

    // Count backticks to see if we have unclosed code blocks
    let backtick_count = text.matches("```").count();

    // If odd number of triple backticks, we have an unclosed block
    if backtick_count % 2 == 1 {
        result.push_str("\n```\n");
    }

    // Add truncation notice
    result.push_str("\n\n*[Output truncated - increase --max-tokens for complete solution]*");

    result
}

/// Validate that a diff actually contains meaningful changes
/// Returns true if the diff is valid (has real changes), false if it's broken
fn validate_diff(solution: &str, original_code: &str) -> bool {
    // Extract diff block from solution
    let diff_start = solution.find("```diff");
    if diff_start.is_none() {
        // No diff found - could be a text explanation saying no optimization possible
        // If solution contains phrases indicating no optimization, consider it valid
        let no_opt_phrases = [
            "no optimization possible",
            "cannot be optimized",
            "already optimal",
            "necessary operations",
        ];
        if no_opt_phrases.iter().any(|phrase| solution.to_lowercase().contains(phrase)) {
            return true; // Valid explanation that no fix is possible
        }
        return false; // No diff and no explanation - invalid
    }

    let diff_start = diff_start.unwrap() + 7; // Skip "```diff\n"
    let diff_end = solution[diff_start..].find("```").unwrap_or(solution.len() - diff_start);
    let diff_text = &solution[diff_start..diff_start + diff_end];

    // Parse the diff to extract added and removed lines
    let mut removed_lines = Vec::new();
    let mut added_lines = Vec::new();

    for line in diff_text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('-') && !trimmed.starts_with("---") {
            // Remove the leading '-' and any whitespace
            let content = trimmed[1..].trim();
            if !content.is_empty() && !content.starts_with("--") {
                removed_lines.push(content);
            }
        } else if trimmed.starts_with('+') && !trimmed.starts_with("+++") {
            // Remove the leading '+' and any whitespace
            let content = trimmed[1..].trim();
            if !content.is_empty() && !content.starts_with("++") {
                added_lines.push(content);
            }
        }
    }

    // Check 1: Must have at least some changes
    if removed_lines.is_empty() && added_lines.is_empty() {
        return false;
    }

    // Check 2: If we have both additions and removals, they shouldn't all be identical
    if !removed_lines.is_empty() && !added_lines.is_empty() {
        // Compare the lines - if every removed line has an identical added line, it's broken
        let mut all_identical = true;
        for removed in &removed_lines {
            if !added_lines.iter().any(|added| {
                // Normalize whitespace for comparison
                normalize_code_line(removed) == normalize_code_line(added)
            }) {
                all_identical = false;
                break;
            }
        }

        if all_identical && removed_lines.len() == added_lines.len() {
            return false; // All lines are identical - broken diff
        }
    }

    // Check 3: Verify removed lines actually exist in original code
    // This catches hallucinated diffs
    if !removed_lines.is_empty() {
        let original_normalized: Vec<String> = original_code
            .lines()
            .map(|l| normalize_code_line(l.trim()))
            .filter(|l| !l.is_empty())
            .collect();

        let mut found_count = 0;
        for removed in &removed_lines {
            let normalized_removed = normalize_code_line(removed);
            if original_normalized.iter().any(|orig| orig.contains(&normalized_removed) || normalized_removed.contains(orig)) {
                found_count += 1;
            }
        }

        // At least 50% of removed lines should exist in original code
        // (allowing some flexibility for partial matches)
        if found_count == 0 && removed_lines.len() > 0 {
            return false; // None of the removed lines exist - hallucinated diff
        }
    }

    true // Diff looks valid
}

/// Normalize a code line for comparison (remove extra whitespace, comments)
fn normalize_code_line(line: &str) -> String {
    line.split('#').next().unwrap_or("")  // Remove comments
        .replace(" ", "")                  // Remove all whitespace
        .replace("\t", "")
        .to_lowercase()
}

/// Extract optimized function from LLM response
fn extract_optimized_function(solution: &str) -> Option<String> {
    // Handle responses that include an opening fence, and tolerate missing closing fence.
    let mut body = solution.trim().to_string();
    if body.starts_with("```") {
        // Drop the opening fence line (e.g., ```python)
        let mut lines = body.lines();
        let _ = lines.next();
        body = lines.collect::<Vec<_>>().join("\n");
        body = body.trim().to_string();
    }

    // If a closing fence exists, stop there; otherwise use full body.
    let end_marker = "```";
    let code_slice = if let Some(end) = body.find(end_marker) {
        &body[..end]
    } else {
        body.as_str()
    };

    let mut code = code_slice.trim().to_string();

    // Remove any import statements that LLM might have added
    let lines: Vec<&str> = code.lines().collect();
    let filtered_lines: Vec<&str> = lines.into_iter()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.starts_with("import ") && !trimmed.starts_with("from ")
        })
        .collect();

    code = filtered_lines.join("\n");

    Some(code.trim().to_string())
}

/// Generate a unified diff from original and optimized code
fn generate_diff(original: &str, optimized: &str) -> String {
    let diff = TextDiff::from_lines(original, optimized);
    let mut result = String::new();

    for change in diff.iter_all_changes() {
        let sign = match change.tag() {
            ChangeTag::Delete => "-",
            ChangeTag::Insert => "+",
            ChangeTag::Equal => " ",
        };
        result.push_str(&format!("{}{}", sign, change));
    }

    result
}

/// Validate that optimized function is substantially different from original
fn validate_optimization(original: &str, optimized: &str) -> Result<(), String> {
    // Must be different
    if original.trim() == optimized.trim() {
        return Err("optimized code is identical to original".to_string());
    }

    // Check that there are actual code changes (not just comment changes)
    let orig_lines: Vec<&str> = original.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();

    let opt_lines: Vec<&str> = optimized.lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .collect();

    // Must have at least some different non-comment lines
    if orig_lines != opt_lines {
        Ok(())
    } else {
        Err("optimized code only changes whitespace/comments".to_string())
    }
}

fn print_summary(file_results: &[FileResults], file_count: usize, total: usize, functions_with_issues: usize, checks: &[CheckConfig], cache: &AnalysisCache, no_cache: bool, stats: &TokenStats) {
    println!("\n╔═══════════════════════════════╗");
    println!("║ LOOPSLEUTH ANALYSIS SUMMARY   ║");
    println!("╚═══════════════════════════════╝");
    println!();

    if file_count > 1 {
        println!("📁 Files analyzed: {}", file_count);
    }
    println!("📊 Total functions analyzed: {}", total);
    println!("🔍 Checks run: {} ({})",
        checks.len(),
        checks.iter().map(|c| c.key.clone()).collect::<Vec<_>>().join(", ")
    );
    println!("⚠️  Functions with issues: {}", functions_with_issues);
    println!("✓  Functions clean: {}", total - functions_with_issues);

    // Show cache statistics if enabled
    if !no_cache {
        if let Ok((cache_total, cache_with_issues)) = cache.stats() {
            if cache_total > 0 {
                let expected_total = total * checks.len();
                println!("💾 Cache entries: {} (expected: {} = {} functions × {} checks), {} with issues",
                    cache_total, expected_total, total, checks.len(), cache_with_issues);
            }
        }
    }

    if functions_with_issues > 0 {
        println!("\n🔴 ISSUES DETECTED:");
        println!("─────────────────────────────────────────────────────────────");

        if file_count > 1 {
            // Group by file when analyzing multiple files
            for file_result in file_results {
                let functions_with_issues_in_file: Vec<_> = file_result.results.iter()
                    .filter(|r| r.check_results.iter().any(|cr| cr.has_issue))
                    .collect();

                if !functions_with_issues_in_file.is_empty() {
                    println!("\n  📄 {}", file_result.file_path.display());
                    for result in functions_with_issues_in_file {
                        let issues: Vec<_> = result.check_results.iter()
                            .filter(|cr| cr.has_issue)
                            .map(|cr| cr.check_name.as_str())
                            .collect();
                        let func_name = if let Some(ref class_name) = result.function.class_name {
                            format!("{}::{}", class_name, result.function.name)
                        } else {
                            result.function.name.clone()
                        };
                        println!(
                            "     • {} (line {})",
                            func_name,
                            result.function.line_number
                        );
                        for issue in issues {
                            println!("       - {}", issue);
                        }
                    }
                }
            }
        } else {
            // Flat list for single file
            for file_result in file_results {
                for result in file_result.results.iter() {
                    let issues: Vec<_> = result.check_results.iter()
                        .filter(|cr| cr.has_issue)
                        .collect();

                    if !issues.is_empty() {
                        let func_name = if let Some(ref class_name) = result.function.class_name {
                            format!("{}::{}", class_name, result.function.name)
                        } else {
                            result.function.name.clone()
                        };
                        println!(
                            "  • {} ({}:{})",
                            func_name,
                            result.function.file_path.display(),
                            result.function.line_number
                        );
                        for issue in issues {
                            println!("    - {}", issue.check_name);
                        }
                    }
                }
            }
        }
    }

    // Show token usage statistics
    if stats.output_tokens > 0 {
        println!();
        println!("📈 Token Usage:");
        println!("   • Input:  {} tokens", stats.input_tokens);
        println!("   • Output: {} tokens", stats.output_tokens);
        println!("   • Speed:  {:.1} tokens/sec", stats.tokens_per_second());
        println!("   • Time:   {:.1}s", stats.generation_time.as_secs_f64());
    }

    println!();
}

fn print_detailed_report(results: &[AnalysisResult]) {
    println!("═══════════════════════════════════════════════════════════════");
    println!("                     DETAILED REPORT");
    println!("═══════════════════════════════════════════════════════════════");
    println!();

    let results_with_issues: Vec<_> = results.iter()
        .filter(|r| r.check_results.iter().any(|cr| cr.has_issue))
        .collect();

    for (idx, result) in results_with_issues.iter().enumerate() {
        let func_name = if let Some(ref class_name) = result.function.class_name {
            format!("{}::{}", class_name, result.function.name)
        } else {
            result.function.name.clone()
        };
        println!("## {} - `{}`", idx + 1, func_name);
        println!();
        println!("**Location:** `{}:{}`",
            result.function.file_path.display(),
            result.function.line_number
        );
        println!();

        println!("### 📝 Original Code");
        println!();
        let highlighted_source = highlight_source_for_issues(&result.function.source, &result.check_results);
        println!("```python");
        println!("{}", highlighted_source);
        println!("```");
        println!("> Note: lines prefixed with '>>' are suspected hotspots.");
        println!();

        // Show all issues for this function
        let issues: Vec<_> = result.check_results.iter().filter(|cr| cr.has_issue).collect();

        for (issue_idx, issue) in issues.iter().enumerate() {
            // Extract confidence from analysis
            let confidence_pct = extract_confidence_percentage(&issue.analysis);

            if issues.len() > 1 {
                println!("### ⚠️ Issue {}: {} (confidence: {}%)", issue_idx + 1, issue.check_name, confidence_pct);
            } else {
                println!("### ⚠️ Issue: {} (confidence: {}%)", issue.check_name, confidence_pct);
            }
            println!();

            if let Some(solution) = &issue.solution {
                // Show full analysis when we have a solution
                println!("{}", issue.analysis.trim());
                println!();
                println!("### 💡 Suggested Optimization");
                println!();
                println!("{}", solution.trim());
                println!();
            }
            // When no solution, just show the simple warning above (no detailed analysis)
        }

        if idx < results_with_issues.len() - 1 {
            println!("───────────────────────────────────────────────────────────────");
            println!();
        }
    }

    println!("═══════════════════════════════════════════════════════════════");
    println!();
    println!("📄 Copy this report to your code review or documentation!");
    println!();
}

fn highlight_source_for_issues(source: &str, check_results: &[CheckResult]) -> String {
    let mut tokens: Vec<String> = Vec::new();
    for issue in check_results.iter().filter(|cr| cr.has_issue) {
        tokens.extend(extract_detail_tokens(&issue.analysis));
    }
    tokens.sort();
    tokens.dedup();

    if tokens.is_empty() {
        return source.to_string();
    }

    let mut out_lines = Vec::new();
    for line in source.lines() {
        if tokens.iter().any(|t| line.contains(t)) {
            out_lines.push(format!(">> {}", line));
        } else {
            out_lines.push(format!("   {}", line));
        }
    }
    out_lines.join("\n")
}

fn highlight_source_html(source: &str, check_results: &[CheckResult]) -> String {
    let mut tokens: Vec<String> = Vec::new();
    for issue in check_results.iter().filter(|cr| cr.has_issue) {
        tokens.extend(extract_detail_tokens(&issue.analysis));
    }
    tokens.sort();
    tokens.dedup();

    if tokens.is_empty() {
        return escape_html(source);
    }

    let mut out_lines = Vec::new();
    for line in source.lines() {
        let escaped = escape_html(line);
        if tokens.iter().any(|t| line.contains(t)) {
            out_lines.push(format!(
                "<span class=\"hotspot\">{}</span>",
                escaped
            ));
        } else {
            out_lines.push(escaped);
        }
    }
    out_lines.join("\n")
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('\"', "&quot;")
        .replace('\'', "&#39;")
}

fn extract_detail_tokens(analysis: &str) -> Vec<String> {
    let detail_line = analysis
        .lines()
        .find(|line| line.trim_start().starts_with("DETAIL:"))
        .map(|line| line.trim_start()["DETAIL:".len()..].trim())
        .unwrap_or("");

    if detail_line.is_empty() {
        return Vec::new();
    }

    let mut tokens = Vec::new();
    let call_re = Regex::new(r"[A-Za-z_][A-Za-z0-9_\.]*\s*\([^)]*\)").unwrap();
    let dotted_re = Regex::new(r"[A-Za-z_][A-Za-z0-9_]*\.[A-Za-z0-9_\.]+").unwrap();

    for cap in call_re.find_iter(detail_line) {
        tokens.push(cap.as_str().trim().to_string());
    }
    for cap in dotted_re.find_iter(detail_line) {
        tokens.push(cap.as_str().trim().to_string());
    }

    // Add simple variants without trailing punctuation to improve matching.
    let mut variants = Vec::new();
    for t in &tokens {
        let trimmed = t.trim_end_matches(&[')', ',', '.'][..]).to_string();
        if !trimmed.is_empty() {
            variants.push(trimmed);
        }
    }
    tokens.extend(variants);
    tokens
}

fn write_report_to_file(
    path: &PathBuf,
    all_results: &[AnalysisResult],
    total: usize,
    functions_with_issues: usize,
    checks: &[CheckConfig],
    cache: &AnalysisCache,
    no_cache: bool,
) -> Result<()> {
    use std::io::Write;

    let mut file = std::fs::File::create(path)?;

    writeln!(file, "<!doctype html>")?;
    writeln!(file, "<html lang=\"en\">")?;
    writeln!(file, "<head>")?;
    writeln!(file, "  <meta charset=\"utf-8\">")?;
    writeln!(file, "  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">")?;
    writeln!(file, "  <title>LoopSleuth Analysis Report</title>")?;
    writeln!(file, "  <style>")?;
    writeln!(file, "    body {{ font-family: -apple-system, BlinkMacSystemFont, Segoe UI, sans-serif; margin: 24px; color: #111; }}")?;
    writeln!(file, "    h1, h2, h3, h4 {{ margin: 16px 0 8px; }}")?;
    writeln!(file, "    .meta {{ color: #555; margin-bottom: 16px; }}")?;
    writeln!(file, "    .summary li {{ margin: 4px 0; }}")?;
    writeln!(file, "    .issue-list li {{ margin: 4px 0; }}")?;
    writeln!(file, "    code, pre {{ font-family: ui-monospace, SFMono-Regular, Menlo, monospace; }}")?;
    writeln!(file, "    pre {{ background: #fafafa; border: 1px solid #eee; padding: 12px; overflow: auto; }}")?;
    writeln!(file, "    .hotspot {{ background-color: #ffe6e6; }}")?;
    writeln!(file, "    .note {{ color: #666; font-size: 0.9em; }}")?;
    writeln!(file, "    hr {{ border: none; border-top: 1px solid #eee; margin: 20px 0; }}")?;
    writeln!(file, "  </style>")?;
    writeln!(file, "</head>")?;
    writeln!(file, "<body>")?;

    writeln!(file, "<h1>LoopSleuth Analysis Report</h1>")?;
    writeln!(
        file,
        "<div class=\"meta\">Generated: {}</div>",
        chrono::Local::now().format("%Y-%m-%d %H:%M:%S")
    )?;

    writeln!(file, "<h2>Summary</h2>")?;
    writeln!(file, "<ul class=\"summary\">")?;
    writeln!(file, "<li><strong>Total functions analyzed:</strong> {}</li>", total)?;
    writeln!(
        file,
        "<li><strong>Checks run:</strong> {} ({})</li>",
        checks.len(),
        checks.iter().map(|c| c.key.clone()).collect::<Vec<_>>().join(", ")
    )?;
    writeln!(file, "<li><strong>Functions with issues:</strong> {}</li>", functions_with_issues)?;
    writeln!(file, "<li><strong>Functions clean:</strong> {}</li>", total - functions_with_issues)?;
    if !no_cache {
        if let Ok((cache_total, cache_with_issues)) = cache.stats() {
            if cache_total > 0 {
                writeln!(
                    file,
                    "<li><strong>Cache entries:</strong> {} total, {} with issues</li>",
                    cache_total,
                    cache_with_issues
                )?;
            }
        }
    }
    writeln!(file, "</ul>")?;

    if functions_with_issues > 0 {
        writeln!(file, "<h2>Issues Detected</h2>")?;
        writeln!(file, "<ul class=\"issue-list\">")?;

        for result in all_results.iter() {
            let issues: Vec<_> = result.check_results.iter().filter(|cr| cr.has_issue).collect();
            if !issues.is_empty() {
                let func_name = if let Some(ref class_name) = result.function.class_name {
                    format!("{}::{}", class_name, result.function.name)
                } else {
                    result.function.name.clone()
                };
                writeln!(
                    file,
                    "<li><code>{}</code> ({}:{})",
                    escape_html(&func_name),
                    escape_html(&result.function.file_path.display().to_string()),
                    result.function.line_number
                )?;
                writeln!(file, "<ul>")?;
                for issue in issues {
                    writeln!(file, "<li>{}</li>", escape_html(&issue.check_name))?;
                }
                writeln!(file, "</ul></li>")?;
            }
        }
        writeln!(file, "</ul>")?;

        writeln!(file, "<hr>")?;
        writeln!(file, "<h2>Detailed Analysis</h2>")?;

        let results_with_issues: Vec<_> = all_results
            .iter()
            .filter(|r| r.check_results.iter().any(|cr| cr.has_issue))
            .collect();

        for (idx, result) in results_with_issues.iter().enumerate() {
            let func_name = if let Some(ref class_name) = result.function.class_name {
                format!("{}::{}", class_name, result.function.name)
            } else {
                result.function.name.clone()
            };
            writeln!(file, "<h3>{} - <code>{}</code></h3>", idx + 1, escape_html(&func_name))?;
            writeln!(
                file,
                "<div><strong>Location:</strong> <code>{}:{}</code></div>",
                escape_html(&result.function.file_path.display().to_string()),
                result.function.line_number
            )?;
            writeln!(file, "<h4>Original Code</h4>")?;
            let highlighted_html = highlight_source_html(&result.function.source, &result.check_results);
            writeln!(file, "<pre><code class=\"language-python\">{}</code></pre>", highlighted_html)?;
            writeln!(file, "<div class=\"note\">Lines with light red background are suspected hotspots.</div>")?;

            let issues: Vec<_> = result.check_results.iter().filter(|cr| cr.has_issue).collect();
            for (issue_idx, issue) in issues.iter().enumerate() {
                let confidence_pct = extract_confidence_percentage(&issue.analysis);
                if issues.len() > 1 {
                    writeln!(
                        file,
                        "<h4>Issue {}: {} (confidence: {}%)</h4>",
                        issue_idx + 1,
                        escape_html(&issue.check_name),
                        confidence_pct
                    )?;
                } else {
                    writeln!(
                        file,
                        "<h4>Issue: {} (confidence: {}%)</h4>",
                        escape_html(&issue.check_name),
                        confidence_pct
                    )?;
                }

                if let Some(solution) = &issue.solution {
                    writeln!(file, "<div><pre><code>{}</code></pre></div>", escape_html(issue.analysis.trim()))?;
                    writeln!(file, "<h4>Suggested Optimization</h4>")?;
                    writeln!(file, "<div><pre><code>{}</code></pre></div>", escape_html(solution.trim()))?;
                }
            }

            if idx < results_with_issues.len() - 1 {
                writeln!(file, "<hr>")?;
            }
        }
    }

    writeln!(file, "<hr>")?;
    writeln!(file, "<div class=\"note\">Generated by LoopSleuth</div>")?;
    writeln!(file, "</body>")?;
    writeln!(file, "</html>")?;

    Ok(())
}
