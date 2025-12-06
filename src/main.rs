use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use globset::{Glob, GlobSetBuilder};
use ignore::WalkBuilder;
use indicatif::{ProgressBar, ProgressStyle};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;
use tokio::sync::Mutex;

// ==========================================
// CLI Configuration
// ==========================================

#[derive(Parser, Debug, Clone)]
#[command(name = "repomix", version, about = "Pack your repository into a single AI-friendly file")]
struct Cli {
    #[arg(default_value = ".")]
    directories: Vec<String>,

    #[arg(short, long)]
    output: Option<String>,

    #[arg(long, value_enum, default_value_t = OutputStyle::Xml)]
    style: OutputStyle,

    #[arg(long)]
    parsable_style: bool,

    #[arg(short, long)]
    config: Option<String>,

    #[arg(long)]
    copy: bool,

    #[arg(long)]
    verbose: bool,

    #[arg(long)]
    top_files_len: Option<usize>,

    #[arg(long)]
    output_show_line_numbers: bool,

    #[arg(long)]
    remove_comments: bool,

    #[arg(long)]
    remove_empty_lines: bool,

    #[arg(long)]
    compress: bool,

    #[arg(long)]
    include_empty_directories: bool,

    #[arg(long)]
    remote: Option<String>,

    #[arg(long)]
    remote_branch: Option<String>,

    #[arg(long)]
    security_check: Option<bool>,

    #[arg(long)]
    include: Option<String>,

    #[arg(short, long)]
    ignore: Option<String>,

    #[arg(long)]
    no_gitignore: bool,

    #[arg(long)]
    no_default_patterns: bool,

    #[arg(long)]
    header_text: Option<String>,

    #[arg(long)]
    instruction_file_path: Option<String>,

    #[arg(long)]
    include_diffs: bool,

    #[arg(long)]
    include_logs: bool,

    // --- NEW ARGUMENTS ---

    /// The specific task you want the LLM to perform.
    /// If provided, this generates a custom prompt at the top of the file.
    #[arg(long)]
    intent: Option<String>,

    /// A comma-separated list of files to include in FULL TEXT, overriding compression.
    /// Example: --focus "src/main.rs,src/utils.rs"
    #[arg(long)]
    focus: Option<String>,
}

#[derive(ValueEnum, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum OutputStyle {
    Xml,
    Markdown,
    Json,
    Plain,
}

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
struct RepomixConfig {
    output: OutputConfig,
    ignore: IgnoreConfig,
    security: SecurityConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default, rename_all = "camelCase")]
struct OutputConfig {
    file_path: String,
    style: OutputStyle,
    top_files_length: usize,
    show_line_numbers: bool,
    remove_comments: bool,
    remove_empty_lines: bool,
    compress: bool,
    copy_to_clipboard: bool,
    header_text: Option<String>,
    instruction_file_path: Option<String>,
    include_empty_directories: bool,
    include_diffs: bool,
    include_logs: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default, rename_all = "camelCase")]
struct IgnoreConfig {
    use_gitignore: bool,
    use_default_patterns: bool,
    custom_patterns: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default, rename_all = "camelCase")]
struct SecurityConfig {
    enable_security_check: bool,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            file_path: "repomix-output.xml".to_string(),
            style: OutputStyle::Xml,
            top_files_length: 5,
            show_line_numbers: false,
            remove_comments: false,
            remove_empty_lines: false,
            compress: false,
            copy_to_clipboard: false,
            header_text: None,
            instruction_file_path: None,
            include_empty_directories: false,
            include_diffs: false,
            include_logs: false,
        }
    }
}

impl Default for IgnoreConfig {
    fn default() -> Self {
        Self {
            use_gitignore: true,
            use_default_patterns: true,
            custom_patterns: vec![],
        }
    }
}

impl Default for SecurityConfig {
    fn default() -> Self {
        Self {
            enable_security_check: true,
        }
    }
}

#[derive(Debug)]
struct ProcessedFile {
    path: String,
    content: String,
    char_count: usize,
    token_count: usize,
    // Track if this file is full text (focus) or skeleton (compressed context)
    is_skeleton: bool, 
}

// ==========================================
// Logic Modules
// ==========================================

mod comments {
    use super::*;

    pub fn remove_comments(content: &str, extension: &str) -> Option<String> {
        let pattern = match extension {
            "rs" | "ts" | "tsx" | "js" | "jsx" | "go" | "java" | "c" | "cpp" | "h" | "hpp" => {
                // C-style comments: // ... and /* ... */
                r"(?s)//.*?\n|/\*.*?\*/"
            },
            "py" | "sh" | "yaml" | "yml" | "toml" | "rb" | "pl" => {
                // Hash-style comments: # ...
                r"#.*"
            },
            _ => return None,
        };

        if let Ok(re) = Regex::new(pattern) {
             Some(re.replace_all(content, "").to_string())
        } else {
             None
        }
    }
}

mod compression {
    use super::*;
    use tree_sitter::{Parser, Query, QueryCursor};
    use streaming_iterator::StreamingIterator;

    pub fn compress_content(content: &str, extension: &str) -> Option<String> {
        let mut parser = Parser::new();
        
        let (language, query_str) = match extension {
            "rs" => (tree_sitter_rust::LANGUAGE.into(), RUST_QUERY),
            "ts" | "tsx" => (tree_sitter_typescript::LANGUAGE_TSX.into(), TS_QUERY),
            "js" | "jsx" => (tree_sitter_javascript::LANGUAGE.into(), JS_QUERY),
            "py" => (tree_sitter_python::LANGUAGE.into(), PYTHON_QUERY),
            "go" => (tree_sitter_go::LANGUAGE.into(), GO_QUERY),
            _ => return None, // Language not supported for compression
        };

        parser.set_language(&language).ok()?;
        let tree = parser.parse(content, None)?;
        let query = Query::new(&language, query_str).ok()?;
        let mut cursor = QueryCursor::new();
        
        // We collect ranges of "essential" code (signatures, headers)
        let mut ranges = Vec::new();
        
        let mut matches = cursor.matches(&query, tree.root_node(), content.as_bytes());
        while let Some(m) = matches.next() {
            for capture in m.captures {
                let node = capture.node;
                ranges.push(node.byte_range());
            }
        }

        if ranges.is_empty() {
            return Some(content.to_string()); // Fallback if no definitions found
        }

        // Sort and merge overlapping ranges
        ranges.sort_by(|a, b| a.start.cmp(&b.start));
        
        let mut merged_ranges = Vec::new();
        let mut current_range = ranges[0].clone();

        for next in ranges.into_iter().skip(1) {
            if next.start <= current_range.end {
                current_range.end = std::cmp::max(current_range.end, next.end);
            } else {
                merged_ranges.push(current_range);
                current_range = next;
            }
        }
        merged_ranges.push(current_range);

        // Reconstruct content
        let mut result = String::new();
        let bytes = content.as_bytes();
        let separator = "\n// ... [implementation details hidden] ...\n";

        for range in merged_ranges {
            let chunk = String::from_utf8_lossy(&bytes[range.start..range.end]);
            if !result.is_empty() {
                result.push_str(separator);
            }
            result.push_str(chunk.trim());
        }

        Some(result)
    }

    // Simplified queries to capture definitions/signatures
    const RUST_QUERY: &str = r#"
        (function_item) @f
        (impl_item) @i
        (struct_item) @s
        (enum_item) @e
        (trait_item) @t
        (mod_item) @m
    "#;

    const TS_QUERY: &str = r#"
        (function_declaration) @f
        (class_declaration) @c
        (interface_declaration) @i
        (type_alias_declaration) @t
        (enum_declaration) @e
        (method_definition) @m
        (abstract_class_declaration) @ac
        (module) @mod
    "#;

    const JS_QUERY: &str = r#"
        (function_declaration) @f
        (class_declaration) @c
        (method_definition) @m
    "#;

    const PYTHON_QUERY: &str = r#"
        (function_definition) @f
        (class_definition) @c
    "#;

    const GO_QUERY: &str = r#"
        (function_declaration) @f
        (method_declaration) @m
        (type_declaration) @t
    "#;
}

mod git {
    use super::*;

    pub fn is_git_repo(path: &Path) -> bool {
        Command::new("git")
            .arg("rev-parse")
            .arg("--is-inside-work-tree")
            .current_dir(path)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    pub fn clone_repo(url: &str, target: &Path, branch: Option<&str>) -> Result<()> {
        let mut cmd = Command::new("git");
        cmd.arg("clone").arg("--depth").arg("1");
        
        if let Some(b) = branch {
            cmd.arg("--branch").arg(b);
        }
        
        cmd.arg(url).arg(target);
        
        let status = cmd.status().context("Failed to execute git clone")?;
        if !status.success() {
            anyhow::bail!("Git clone failed");
        }
        Ok(())
    }

    pub fn get_diffs(path: &Path) -> Result<String> {
        let output = Command::new("git")
            .args(["diff", "HEAD"])
            .current_dir(path)
            .output()?;
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub fn get_logs(path: &Path) -> Result<String> {
        let output = Command::new("git")
            .args(["log", "-n", "50", "--pretty=format:%h - %an, %ar : %s"])
            .current_dir(path)
            .output()?;
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }

    pub fn get_file_change_counts(path: &Path) -> HashMap<String, usize> {
        let output = Command::new("git")
            .args(["log", "--name-only", "--format=", "-n", "100"])
            .current_dir(path)
            .output();

        let mut counts = HashMap::new();
        if let Ok(out) = output {
            let s = String::from_utf8_lossy(&out.stdout);
            for line in s.lines() {
                if !line.trim().is_empty() {
                    *counts.entry(line.trim().to_string()).or_insert(0) += 1;
                }
            }
        }
        counts
    }
}

mod security {
    use super::*;

    pub fn is_suspicious(content: &str) -> bool {
        let patterns = [
            r#"(?i)(api_key|apikey|secret|token).{0,20}['|"][0-9a-zA-Z]{32,45}['|"]"#,
            r"ghp_[0-9a-zA-Z]{36}",
            r"sk_live_[0-9a-zA-Z]{24}",
        ];
        
        for p in patterns {
            if let Ok(re) = Regex::new(p) {
                if re.is_match(content) {
                    return true;
                }
            }
        }
        false
    }
}

mod fs_tools {
    use super::*;
    use tiktoken_rs::cl100k_base;

    pub fn count_tokens(content: &str) -> usize {
        let bpe = cl100k_base().unwrap();
        bpe.encode_with_special_tokens(content).len()
    }

    pub fn is_binary(content: &[u8]) -> bool {
        let len = std::cmp::min(content.len(), 8192);
        content[0..len].contains(&0)
    }
}

// ==========================================
// Main Logic
// ==========================================

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // 1. Setup Config
    let mut config = RepomixConfig::default();
    
    let config_path = cli.config.clone().unwrap_or_else(|| "repomix.config.json".to_string());
    if Path::new(&config_path).exists() {
        let content = fs::read_to_string(&config_path)?;
        match serde_json::from_str::<RepomixConfig>(&content) {
            Ok(file_config) => {
                if cli.verbose { println!("Loaded config from {}", config_path); }
                config = file_config;
            },
            Err(e) => {
                 if cli.verbose { eprintln!("Failed to parse config {}: {}", config_path, e); }
            }
        }
    } else if cli.verbose {
         println!("Config file {} not found", config_path);
    }

    // --- ARGUMENT PARSING & OVERRIDES ---
    if let Some(s) = &cli.output { config.output.file_path = s.clone(); }
    if cli.style != OutputStyle::Xml { config.output.style = cli.style; }
    if cli.copy { config.output.copy_to_clipboard = true; }
    if let Some(n) = cli.top_files_len { config.output.top_files_length = n; }
    if cli.output_show_line_numbers { config.output.show_line_numbers = true; }
    if cli.remove_comments { config.output.remove_comments = true; }
    if cli.remove_empty_lines { config.output.remove_empty_lines = true; }
    if cli.compress { config.output.compress = true; }
    if cli.include_empty_directories { config.output.include_empty_directories = true; }
    if cli.include_diffs { config.output.include_diffs = true; }
    if cli.include_logs { config.output.include_logs = true; }
    if let Some(h) = cli.header_text { config.output.header_text = Some(h); }
    if let Some(i) = cli.instruction_file_path { config.output.instruction_file_path = Some(i); }
    
    if let Some(sec) = cli.security_check {
        config.security.enable_security_check = sec;
    }
    
    if cli.no_gitignore { config.ignore.use_gitignore = false; }
    if cli.no_default_patterns { config.ignore.use_default_patterns = false; }
    
    if let Some(ign) = cli.ignore {
        config.ignore.custom_patterns.extend(ign.split(',').map(|s| s.to_string()));
    }

    // --- PROMPT INJECTION LOGIC ---
    let mut generated_header = String::new();
    let has_focus = cli.focus.is_some();
    
    if let Some(intent) = &cli.intent {
        if !has_focus {
            // PHASE 1: SURVEY (No focus provided, so we are asking for a plan)
            generated_header.push_str("\n");
            generated_header.push_str("<instruction>\n");
            generated_header.push_str(&format!("THE USER WANTS TO: {}\n\n", intent));
            generated_header.push_str("Attached is the SKELETON of the codebase.\n");
            generated_header.push_str("Your job is to analyze this structure and identify which files are crucial to implement the request.\n");
            generated_header.push_str("RETURN A COMMA-SEPARATED LIST of file paths that must be read in full text.\n");
            generated_header.push_str("Example output: src/auth/login.ts,src/database/models.rs\n");
            generated_header.push_str("</instruction>\n");
        } else {
             // PHASE 2: BUILD (Focus provided, so we are executing)
             generated_header.push_str("\n");
             generated_header.push_str("<instruction>\n");
             generated_header.push_str(&format!("THE USER WANTS TO: {}\n\n", intent));
             generated_header.push_str("Attached is the CONTEXT PACK.\n");
             generated_header.push_str("- Files marked 'mode=\"full\"' are the specific files you requested.\n");
             generated_header.push_str("- Files marked 'mode=\"skeleton\"' are compressed context to prevent hallucinations.\n");
             generated_header.push_str("Please implement the requested changes based on this context.\n");
             generated_header.push_str("</instruction>\n");
        }
        
        // Append to existing header text if any
        if let Some(existing) = config.output.header_text {
             config.output.header_text = Some(format!("{}\n{}", existing, generated_header));
        } else {
             config.output.header_text = Some(generated_header);
        }
    }

    // --- FOCUS LOGIC ---
    // Build a GlobSet for focused files
    let mut focus_set_builder = GlobSetBuilder::new();
    let has_focus_patterns = if let Some(focus_str) = &cli.focus {
        for pattern in focus_str.split(',') {
            if let Ok(glob) = Glob::new(pattern.trim()) {
                focus_set_builder.add(glob);
            }
        }
        true
    } else {
        false
    };
    let focus_set = focus_set_builder.build()?;

    // 2. Handle Remote
    let temp_dir = tempfile::tempdir()?;
    let mut root_paths = Vec::new();

    if let Some(remote_url) = &cli.remote {
        let target = temp_dir.path().join("repo");
        println!("Cloning remote repository...");
        git::clone_repo(remote_url, &target, cli.remote_branch.as_deref())?;
        root_paths.push(target);
    } else {
        for d in &cli.directories {
             if let Ok(canon) = fs::canonicalize(d) {
                root_paths.push(canon);
             } else {
                root_paths.push(PathBuf::from(d));
             }
        }
    }

    // 3. File Discovery
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(ProgressStyle::default_spinner().template("{spinner} {msg}")?);
    spinner.set_message("Searching files...");
    spinner.enable_steady_tick(std::time::Duration::from_millis(100));

    let mut builder = WalkBuilder::new(&root_paths[0]);
    for p in root_paths.iter().skip(1) {
        builder.add(p);
    }

    builder.git_ignore(config.ignore.use_gitignore);

    if config.ignore.use_default_patterns {
        builder.add_custom_ignore_filename(".repomixignore");
    }

    let mut overrides = ignore::overrides::OverrideBuilder::new(&root_paths[0]);

    for pattern in &config.ignore.custom_patterns {
        overrides.add(pattern)?;
    }

    if let Some(inc) = &cli.include {
        for pattern in inc.split(',') {
             overrides.add(&format!("!{}", pattern))?;
        }
    }
    
    builder.overrides(overrides.build()?);

    // Prepare manual globset for ignore patterns to ensure they work reliably
    let mut glob_builder = GlobSetBuilder::new();
    for pattern in &config.ignore.custom_patterns {
        if let Ok(glob) = Glob::new(pattern) {
            glob_builder.add(glob);
        }
    }
    let custom_ignore_set = glob_builder.build()?;

    let walker = builder.build();
    let mut files_to_process = Vec::new();

    for result in walker {
        match result {
            Ok(entry) => {
                if entry.file_type().map(|ft| ft.is_file()).unwrap_or(false) {
                    let path = entry.into_path();

                    // Manual check against custom ignore patterns
                    // We check path relative to the root base
                    let rel_path = pathdiff::diff_paths(&path, &root_paths[0]).unwrap_or_else(|| path.clone());
                    if custom_ignore_set.is_match(&rel_path) {
                        continue;
                    }

                    files_to_process.push(path);
                }
            }
            Err(err) => if cli.verbose { eprintln!("Error walking: {}", err) },
        }
    }

    spinner.set_message(format!("Found {} files. Processing...", files_to_process.len()));

    // 4. Processing
    let processed_files = Arc::new(Mutex::new(Vec::new()));
    let mut tasks = Vec::new();
    let root_base = root_paths[0].clone();

    for path in files_to_process {
        let config = config.clone();
        let processed_files = processed_files.clone();
        let root_base = root_base.clone();
        let focus_set = focus_set.clone();

        tasks.push(tokio::spawn(async move {
            if let Ok(content_bytes) = fs::read(&path) {
                if fs_tools::is_binary(&content_bytes) {
                    return;
                }

                let mut content = String::from_utf8_lossy(&content_bytes).to_string();
                let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

                if config.security.enable_security_check && security::is_suspicious(&content) {
                    return;
                }

                let rel_path = pathdiff::diff_paths(&path, &root_base)
                    .unwrap_or(path.clone())
                    .to_string_lossy()
                    .replace("\\", "/");

                // --- HYBRID COMPRESSION DECISION ---
                // If focus patterns exist:
                //   - If match: Full Text (is_skeleton = false)
                //   - If NO match: Compress it (is_skeleton = true)
                // If NO focus patterns exist:
                //   - Follow global config.compress setting
                
                let is_focused = has_focus_patterns && focus_set.is_match(&rel_path);
                let should_compress_file = if has_focus_patterns {
                    !is_focused // If focus exists, compress everything NOT focused
                } else {
                    config.output.compress // Fallback to global flag
                };

                if should_compress_file {
                    if let Some(compressed) = compression::compress_content(&content, ext) {
                        content = compressed;
                    }
                }

                if config.output.remove_comments {
                    if let Some(stripped) = comments::remove_comments(&content, ext) {
                        content = stripped;
                    }
                }

                if config.output.remove_empty_lines {
                    content = content.lines()
                        .filter(|l| !l.trim().is_empty())
                        .collect::<Vec<_>>()
                        .join("\n");
                }

                if config.output.show_line_numbers {
                    content = content.lines().enumerate()
                        .map(|(i, l)| format!("{:4}: {}", i + 1, l))
                        .collect::<Vec<_>>()
                        .join("\n");
                }

                let token_count = fs_tools::count_tokens(&content);
                let char_count = content.chars().count();
                
                let mut pf = processed_files.lock().await;
                pf.push(ProcessedFile {
                    path: rel_path,
                    content,
                    char_count,
                    token_count,
                    is_skeleton: should_compress_file,
                });
            }
        }));
    }

    for task in tasks {
        let _ = task.await;
    }

    spinner.finish_with_message("Processing complete.");

    // 5. Sorting & Git
    let mut files = Arc::try_unwrap(processed_files).unwrap().into_inner();
    
    if git::is_git_repo(&root_paths[0]) {
        let counts = git::get_file_change_counts(&root_paths[0]);
        files.sort_by(|a, b| {
            let count_a = counts.get(&a.path).unwrap_or(&0);
            let count_b = counts.get(&b.path).unwrap_or(&0);
            count_a.cmp(count_b) 
        });
    } else {
        files.sort_by(|a, b| a.path.cmp(&b.path));
    }

    let git_diff = if config.output.include_diffs {
        git::get_diffs(&root_paths[0]).ok()
    } else { None };

    let git_log = if config.output.include_logs {
        git::get_logs(&root_paths[0]).ok()
    } else { None };

    // 6. Generate Output
    let total_tokens: usize = files.iter().map(|f| f.token_count).sum();
    let output_string = generate_output(&files, &config, git_diff.as_deref(), git_log.as_deref(), total_tokens);

    // 7. Write / Clipboard
    if config.output.copy_to_clipboard {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
             let _ = clipboard.set_text(&output_string);
             println!("Output copied to clipboard!");
        }
    }

    if cli.output.as_deref() == Some("-") {
        print!("{}", output_string);
    } else {
        let out_path = PathBuf::from(&config.output.file_path);
        if let Some(parent) = out_path.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(&out_path, &output_string)?;
        println!("Output written to {}", out_path.display());
    }

    println!("Total Files: {}", files.len());
    println!("Total Tokens: {}", total_tokens);

    Ok(())
}

fn generate_output(
    files: &[ProcessedFile], 
    config: &RepomixConfig, 
    git_diff: Option<&str>,
    git_log: Option<&str>,
    _total_tokens: usize
) -> String {
    match config.output.style {
        OutputStyle::Xml => generate_xml(files, config, git_diff, git_log),
        OutputStyle::Markdown => generate_markdown(files, config, git_diff, git_log),
        OutputStyle::Json => generate_json(files, config, git_diff, git_log),
        OutputStyle::Plain => generate_plain(files, config, git_diff, git_log),
    }
}

fn generate_xml(files: &[ProcessedFile], config: &RepomixConfig, diff: Option<&str>, log: Option<&str>) -> String {
    let mut out = String::new();
    out.push_str("<repomix>\n");
    
    if let Some(h) = &config.output.header_text {
        out.push_str(&format!("<header>{}</header>\n", h));
    }

    out.push_str("<summary>\n");
    out.push_str("  This file is a merged representation of the codebase.\n");
    if let Some(inst) = &config.output.instruction_file_path {
        if let Ok(c) = fs::read_to_string(inst) {
            out.push_str(&format!("<instruction>{}</instruction>\n", c));
        }
    }
    out.push_str("</summary>\n");

    out.push_str("<directory_structure>\n");
    for f in files {
        out.push_str(&format!("  {}\n", f.path));
    }
    out.push_str("</directory_structure>\n");

    out.push_str("<files>\n");
    for f in files {
        let mode = if f.is_skeleton { "skeleton" } else { "full" };
        out.push_str(&format!("<file path=\"{}\" mode=\"{}\">\n", f.path, mode));
        let content = f.content.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;");
        out.push_str(&content);
        out.push_str("\n</file>\n");
    }
    out.push_str("</files>\n");

    if let Some(d) = diff {
        out.push_str("<git_diff>\n");
        out.push_str(d);
        out.push_str("\n</git_diff>\n");
    }

    if let Some(l) = log {
        out.push_str("<git_log>\n");
        out.push_str(l);
        out.push_str("\n</git_log>\n");
    }

    out.push_str("</repomix>");
    out
}

fn generate_markdown(files: &[ProcessedFile], config: &RepomixConfig, diff: Option<&str>, log: Option<&str>) -> String {
    let mut out = String::new();
    
    if let Some(h) = &config.output.header_text {
        out.push_str(&format!("# {}\n\n", h));
    }

    out.push_str("# File Summary\n\n");
    out.push_str("This file is a merged representation of the codebase.\n\n");

    out.push_str("# Directory Structure\n\n```\n");
    for f in files {
        out.push_str(&format!("{}\n", f.path));
    }
    out.push_str("```\n\n");

    out.push_str("# Files\n\n");
    for f in files {
        let mode = if f.is_skeleton { "SKELETON (Context Only)" } else { "FULL TEXT" };
        out.push_str(&format!("## File: {} [{}]\n", f.path, mode));
        let ext = Path::new(&f.path).extension().and_then(|s| s.to_str()).unwrap_or("");
        out.push_str(&format!("```{}\n", ext));
        out.push_str(&f.content);
        out.push_str("\n```\n\n");
    }

    if let Some(d) = diff {
        out.push_str("# Git Diff\n\n```diff\n");
        out.push_str(d);
        out.push_str("\n```\n\n");
    }

    if let Some(l) = log {
        out.push_str("# Git Log\n\n");
        out.push_str(l);
        out.push_str("\n\n");
    }

    out
}

fn generate_plain(files: &[ProcessedFile], config: &RepomixConfig, diff: Option<&str>, log: Option<&str>) -> String {
    let mut out = String::new();
    let sep = "=".repeat(40);
    
    out.push_str(&format!("{}\nREPOMIX OUTPUT\n{}\n\n", sep, sep));
    
    if let Some(h) = &config.output.header_text {
        out.push_str(&format!("HEADER\n{}\n\n", h));
    }

    for f in files {
        out.push_str(&format!("File: {}\n{}\n", f.path, "-".repeat(20)));
        out.push_str(&f.content);
        out.push_str("\n\n");
    }
    
    if let Some(d) = diff {
        out.push_str(&format!("GIT DIFF\n{}\n{}\n\n", "-".repeat(20), d));
    }

    if let Some(l) = log {
        out.push_str(&format!("GIT LOG\n{}\n{}\n\n", "-".repeat(20), l));
    }

    out
}

fn generate_json(files: &[ProcessedFile], _config: &RepomixConfig, diff: Option<&str>, log: Option<&str>) -> String {
    #[derive(Serialize)]
    struct JsonOutput<'a> {
        files: HashMap<&'a String, &'a String>,
        git_diff: Option<&'a str>,
        git_log: Option<&'a str>,
    }

    let mut file_map = HashMap::new();
    for f in files {
        file_map.insert(&f.path, &f.content);
    }

    let output = JsonOutput {
        files: file_map,
        git_diff: diff,
        git_log: log,
    };

    serde_json::to_string_pretty(&output).unwrap_or_default()
}
