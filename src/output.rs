use std::collections::HashMap;
use std::fs;
use std::path::Path;
use serde::Serialize;
use crate::config::RustymixConfig;
use crate::cli::OutputStyle;

#[derive(Debug)]
pub struct ProcessedFile {
    pub path: String,
    pub content: String,
    pub char_count: usize,
    pub token_count: usize,
    // Track if this file is full text (focus) or skeleton (compressed context)
    pub is_skeleton: bool,
}

pub fn generate_output(
    files: &[ProcessedFile],
    config: &RustymixConfig,
    git_diff: Option<&str>,
    git_log: Option<&str>
) -> String {
    match config.output.style {
        OutputStyle::Xml => generate_xml(files, config, git_diff, git_log),
        OutputStyle::Markdown => generate_markdown(files, config, git_diff, git_log),
        OutputStyle::Json => generate_json(files, config, git_diff, git_log),
        OutputStyle::Plain => generate_plain(files, config, git_diff, git_log),
    }
}

fn generate_xml(files: &[ProcessedFile], config: &RustymixConfig, diff: Option<&str>, log: Option<&str>) -> String {
    let mut out = String::new();
    out.push_str("<rustymix>\n");

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

    out.push_str("</rustymix>");
    out
}

fn generate_markdown(files: &[ProcessedFile], config: &RustymixConfig, diff: Option<&str>, log: Option<&str>) -> String {
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

fn generate_plain(files: &[ProcessedFile], config: &RustymixConfig, diff: Option<&str>, log: Option<&str>) -> String {
    let mut out = String::new();
    let sep = "=".repeat(40);

    out.push_str(&format!("{}\nRUSTYMIX OUTPUT\n{}\n\n", sep, sep));

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

fn generate_json(files: &[ProcessedFile], _config: &RustymixConfig, diff: Option<&str>, log: Option<&str>) -> String {
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
