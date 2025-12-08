use anyhow::Result;
use clap::Parser;
use globset::{Glob, GlobSetBuilder};
use ignore::WalkBuilder;
use indicatif::{ProgressBar, ProgressStyle};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::Mutex;

mod cli;
mod config;
mod fs_tools;
mod git;
mod language;
mod output;
mod security;

use cli::{Cli, OutputStyle};
use config::RustymixConfig;
use output::ProcessedFile;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // 1. Setup Config
    let mut config = RustymixConfig::default();
    
    let config_path = cli.config.clone().unwrap_or_else(|| "rustymix.config.json".to_string());
    if Path::new(&config_path).exists() {
        let content = fs::read_to_string(&config_path)?;
        match serde_json::from_str::<RustymixConfig>(&content) {
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

    // --- INTENT COLLECTION ---
    // We collect a list of (intent_name, intent_content) tuples.
    // If CLI intent is a directory, we populate this list.
    // If CLI intent is a file, we populate with one item.
    // If CLI intent is a string, we populate with one item.
    // If no intent, list is empty (default behavior).
    
    struct IntentTask {
        name: String,
        content: String,
    }

    let mut intent_tasks = Vec::new();
    let has_focus = cli.focus.is_some();
    let mut is_bulk_mode = false;

    if let Some(intent_arg) = &cli.intent {
        let path = Path::new(intent_arg);
        if path.is_dir() {
            // Bulk mode
            is_bulk_mode = true;
            let entries = fs::read_dir(path)?;
            for entry in entries {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    let name = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
                    let content = fs::read_to_string(&path)?;
                    intent_tasks.push(IntentTask { name, content });
                }
            }
        } else if path.is_file() {
            // File mode
            let name = path.file_stem().unwrap_or_default().to_string_lossy().to_string();
            let content = fs::read_to_string(path)?;
            intent_tasks.push(IntentTask { name, content });
        } else {
            // Raw string mode
            intent_tasks.push(IntentTask {
                name: "default".to_string(),
                content: intent_arg.clone()
            });
        }
    }

    // --- REPO ANALYSIS (Perform once) ---
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

    // Focus Logic
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
        builder.add_custom_ignore_filename(".rustymixignore");
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
                let is_focused = has_focus_patterns && focus_set.is_match(&rel_path);
                let should_compress_file = if has_focus_patterns {
                    !is_focused
                } else {
                    config.output.compress
                };

                if should_compress_file {
                    if let Some(compressed) = language::compression::compress_content(&content, ext) {
                        content = compressed;
                    }
                }

                if config.output.remove_comments {
                    if let Some(stripped) = language::comments::remove_comments(&content, ext) {
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

    
    // --- OUTPUT GENERATION LOOP ---
    
    // If no intents, we run once with default config
    if intent_tasks.is_empty() {
        intent_tasks.push(IntentTask { name: "default".to_string(), content: String::new() });
    }

    let total_tokens: usize = files.iter().map(|f| f.token_count).sum();
    let multi_output = intent_tasks.len() > 1 || is_bulk_mode;

    for task in &intent_tasks {
        let mut task_config = config.clone();

        // Construct header with intent
        let mut generated_header = String::new();
        if !task.content.is_empty() {
            if !has_focus {
                // PHASE 1: SURVEY
                generated_header.push_str("\n");
                generated_header.push_str("<instruction>\n");
                generated_header.push_str(&format!("THE USER WANTS TO: {}\n\n", task.content));
                generated_header.push_str("Attached is the SKELETON of the codebase.\n");
                generated_header.push_str("Your job is to analyze this structure and identify which files are crucial to implement the request.\n");
                generated_header.push_str("You are a Context Engineer. Your goal is to construct the CLI command for the next phase (Phase 2) that carefully isolates the relevant code while excluding noise.\n\n");
                generated_header.push_str("## Tool Reference: rustymix\n");
                generated_header.push_str("rustymix packs a codebase into a single context file.\n");
                generated_header.push_str("- `--focus \"pattern1,pattern2\"`: Critical files/directories to read in FULL TEXT. Supports globs (e.g., `src/core/**`).\n");
                generated_header.push_str("- `--ignore \"pattern1,pattern2\"`: Files/directories to completely EXCLUDE from the pack (e.g., `tests/**`, `legacy_crate/**`).\n\n");
                generated_header.push_str("## Strategy\n");
                generated_header.push_str("- Use globs (`**`) to select entire relevant directories.\n");
                generated_header.push_str("- Exclude unrelated crates or directories to save tokens.\n");
                generated_header.push_str("- Focus on interfaces and definitions first if the task is exploratory.\n\n");
                generated_header.push_str("## Task\n");
                generated_header.push_str("Based on the user's intent and the attached skeleton, return a SINGLE LINE containing the optimized `rustymix` command arguments.\n");
                generated_header.push_str("Example: `--focus \"src/auth/**,src/main.rs\" --ignore \"tests/**\"`\n");
                generated_header.push_str("DO NOT provide explanations. Just the arguments.\n");
                generated_header.push_str("</instruction>\n");
            } else {
                 // PHASE 2: BUILD
                 generated_header.push_str("\n");
                 generated_header.push_str("<instruction>\n");
                 generated_header.push_str(&format!("THE USER WANTS TO: {}\n\n", task.content));
                 generated_header.push_str("Attached is the CONTEXT PACK.\n");
                 generated_header.push_str("- Files marked 'mode=\"full\"' are the specific files you requested.\n");
                 generated_header.push_str("- Files marked 'mode=\"skeleton\"' are compressed context to prevent hallucinations.\n");
                 generated_header.push_str("Please implement the requested changes based on this context.\n");
                 generated_header.push_str("</instruction>\n");
            }
        }

        if let Some(existing) = task_config.output.header_text {
             task_config.output.header_text = Some(format!("{}\n{}", existing, generated_header));
        } else if !generated_header.is_empty() {
             task_config.output.header_text = Some(generated_header);
        }

        let output_string = output::generate_output(&files, &task_config, git_diff.as_deref(), git_log.as_deref());

        // Determine output path
        let out_path = if multi_output {
             // If multiple intents, we likely want to output to a specific directory or format filenames
             // "rustymix-output-intentName.xml"
             let base_dir = if let Some(out_arg) = &cli.output {
                 if Path::new(out_arg).is_dir() {
                     PathBuf::from(out_arg)
                 } else {
                     // If output arg is a file but we have multiple outputs, we fallback to parent dir
                     PathBuf::from(out_arg).parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("."))
                 }
             } else {
                 PathBuf::from(".")
             };

             let ext = match task_config.output.style {
                 OutputStyle::Xml => "xml",
                 OutputStyle::Markdown => "md",
                 OutputStyle::Json => "json",
                 OutputStyle::Plain => "txt",
             };

             base_dir.join(format!("rustymix-{}.{}", task.name, ext))
        } else {
             PathBuf::from(&task_config.output.file_path)
        };

        // Write
        if task_config.output.copy_to_clipboard && !multi_output {
             if let Ok(mut clipboard) = arboard::Clipboard::new() {
                 let _ = clipboard.set_text(&output_string);
                 println!("Output copied to clipboard!");
             }
        }

        if cli.output.as_deref() == Some("-") && !multi_output {
            print!("{}", output_string);
        } else {
            if let Some(parent) = out_path.parent() {
                fs::create_dir_all(parent)?;
            }
            fs::write(&out_path, &output_string)?;
            println!("Output written to {}", out_path.display());
        }
    }

    if multi_output {
         println!("Processed {} intents.", intent_tasks.len());
    }

    println!("Total Files: {}", files.len());
    println!("Total Tokens: {}", total_tokens);

    Ok(())
}
