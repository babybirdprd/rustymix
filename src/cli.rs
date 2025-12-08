use clap::{Parser, ValueEnum};
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug, Clone)]
#[command(name = "rustymix", version, about = "Pack your repository into a single AI-friendly file")]
pub struct Cli {
    /// List of directories to process. Defaults to the current directory (".").
    #[arg(default_value = ".")]
    pub directories: Vec<String>,

    /// The output file path. If not specified, output may go to stdout or be generated based on intent.
    #[arg(short, long)]
    pub output: Option<String>,

    /// The output format style.
    #[arg(long, value_enum, default_value_t = OutputStyle::Xml)]
    pub style: OutputStyle,

    /// [Deprecated] Enable a machine-parsable style if applicable.
    #[arg(long)]
    pub parsable_style: bool,

    /// Path to a specific configuration file (e.g., rustymix.config.json).
    #[arg(short, long)]
    pub config: Option<String>,

    /// Copy the generated output to the system clipboard.
    #[arg(long)]
    pub copy: bool,

    /// Enable verbose logging for debugging purposes.
    #[arg(long)]
    pub verbose: bool,

    /// The number of "top files" to display in the summary (based on some metric like modification count).
    #[arg(long)]
    pub top_files_len: Option<usize>,

    /// Add line numbers to the source code in the output.
    #[arg(long)]
    pub output_show_line_numbers: bool,

    /// Remove comments from the source code (supported languages only).
    #[arg(long)]
    pub remove_comments: bool,

    /// Remove empty lines to compact the code.
    #[arg(long)]
    pub remove_empty_lines: bool,

    /// aggressively compress the code (remove extra whitespace, newlines, etc.).
    #[arg(long)]
    pub compress: bool,

    /// Include empty directories in the file listing.
    #[arg(long)]
    pub include_empty_directories: bool,

    /// A remote repository URL to clone and process.
    #[arg(long)]
    pub remote: Option<String>,

    /// The branch to check out for the remote repository.
    #[arg(long)]
    pub remote_branch: Option<String>,

    /// Enable or disable the security check for suspicious content (e.g. secrets).
    #[arg(long)]
    pub security_check: Option<bool>,

    /// Additional glob patterns to include (overriding ignores).
    #[arg(long)]
    pub include: Option<String>,

    /// Additional glob patterns to ignore.
    #[arg(short, long)]
    pub ignore: Option<String>,

    /// Disable the use of .gitignore files.
    #[arg(long)]
    pub no_gitignore: bool,

    /// Disable default ignore patterns entry (e.g. .git, node_modules).
    #[arg(long)]
    pub no_default_patterns: bool,

    /// Custom text to include in the header of the output.
    #[arg(long)]
    pub header_text: Option<String>,

    /// Path to a file containing instructions/text to include in the header.
    #[arg(long)]
    pub instruction_file_path: Option<String>,

    /// Include git diffs in the output (if in a git repository).
    #[arg(long)]
    pub include_diffs: bool,

    /// Include git log history in the output (if in a git repository).
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
