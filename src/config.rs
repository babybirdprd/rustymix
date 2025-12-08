use serde::{Deserialize, Serialize};
use crate::cli::OutputStyle;

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
#[serde(default)]
pub struct RustymixConfig {
    pub output: OutputConfig,
    pub ignore: IgnoreConfig,
    pub security: SecurityConfig,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct OutputConfig {
    pub file_path: String,
    pub style: OutputStyle,
    pub top_files_length: usize,
    pub show_line_numbers: bool,
    pub remove_comments: bool,
    pub remove_empty_lines: bool,
    pub compress: bool,
    pub copy_to_clipboard: bool,
    pub header_text: Option<String>,
    pub instruction_file_path: Option<String>,
    pub include_empty_directories: bool,
    pub include_diffs: bool,
    pub include_logs: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct IgnoreConfig {
    pub use_gitignore: bool,
    pub use_default_patterns: bool,
    pub custom_patterns: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(default, rename_all = "camelCase")]
pub struct SecurityConfig {
    pub enable_security_check: bool,
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self {
            file_path: "rustymix-output.xml".to_string(),
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
