use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug, Clone)]
#[command(name = "repomix", version, about = "Pack your repository into a single AI-friendly file")]
pub struct Cli {
    #[arg(default_value = ".")]
    pub directories: Vec<String>,

    #[arg(short, long)]
    pub output: Option<String>,

    #[arg(long, value_enum, default_value_t = OutputStyle::Xml)]
    pub style: OutputStyle,

    #[arg(long)]
    pub parsable_style: bool,

    #[arg(short, long)]
    pub config: Option<String>,

    #[arg(long)]
    pub copy: bool,

    #[arg(long)]
    pub verbose: bool,

    #[arg(long)]
    pub top_files_len: Option<usize>,

    #[arg(long)]
    pub output_show_line_numbers: bool,

    #[arg(long)]
    pub remove_comments: bool,

    #[arg(long)]
    pub remove_empty_lines: bool,

    #[arg(long)]
    pub compress: bool,

    #[arg(long)]
    pub include_empty_directories: bool,

    #[arg(long)]
    pub remote: Option<String>,

    #[arg(long)]
    pub remote_branch: Option<String>,

    #[arg(long)]
    pub security_check: Option<bool>,

    #[arg(long)]
    pub include: Option<String>,

    #[arg(short, long)]
    pub ignore: Option<String>,

    #[arg(long)]
    pub no_gitignore: bool,

    #[arg(long)]
    pub no_default_patterns: bool,

    #[arg(long)]
    pub header_text: Option<String>,

    #[arg(long)]
    pub instruction_file_path: Option<String>,

    #[arg(long)]
    pub include_diffs: bool,

    #[arg(long)]
    pub include_logs: bool,

    // --- NEW ARGUMENTS ---

    /// The specific task you want the LLM to perform.
    /// If provided, this generates a custom prompt at the top of the file.
    #[arg(long)]
    pub intent: Option<String>,

    /// A comma-separated list of files to include in FULL TEXT, overriding compression.
    /// Example: --focus "src/main.rs,src/utils.rs"
    #[arg(long)]
    pub focus: Option<String>,
}

#[derive(ValueEnum, Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum OutputStyle {
    Xml,
    Markdown,
    Json,
    Plain,
}
