use assert_cmd::Command;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

// --- Helper Functions to Setup Test Repos ---

fn init_git_repo(path: &Path) {
    std::process::Command::new("git")
        .arg("init")
        .current_dir(path)
        .output()
        .expect("Failed to git init");

    std::process::Command::new("git")
        .arg("config")
        .arg("user.email")
        .arg("test@example.com")
        .current_dir(path)
        .output()
        .expect("Failed to set user.email");

    std::process::Command::new("git")
        .arg("config")
        .arg("user.name")
        .arg("Test User")
        .current_dir(path)
        .output()
        .expect("Failed to set user.name");

    std::process::Command::new("git")
        .arg("add")
        .arg(".")
        .current_dir(path)
        .output()
        .expect("Failed to git add");

    std::process::Command::new("git")
        .arg("commit")
        .arg("-m")
        .arg("Initial commit")
        .current_dir(path)
        .output()
        .expect("Failed to git commit");
}

fn create_rust_repo(parent: &Path) -> PathBuf {
    let repo_path = parent.join("rust_repo");
    fs::create_dir_all(repo_path.join("src")).unwrap();

    let main_rs = r#"fn main() {
    println!("Hello, world!");
}

struct TestStruct {
    field: i32,
}

impl TestStruct {
    fn new() -> Self {
        Self { field: 0 }
    }
}
// This is a comment
"#;
    fs::write(repo_path.join("src/main.rs"), main_rs).unwrap();
    init_git_repo(&repo_path);
    repo_path
}

fn create_ts_repo(parent: &Path) -> PathBuf {
    let repo_path = parent.join("ts_repo");
    fs::create_dir_all(repo_path.join("src")).unwrap();

    let index_ts = r#"interface User {
  id: number;
  name: string;
}

class UserManager {
  constructor(private users: User[]) {}

  getUser(id: number): User | undefined {
    // Return user
    return this.users.find(u => u.id === id);
  }
}

function helper() {
  console.log("Helper");
}
"#;
    fs::write(repo_path.join("src/index.ts"), index_ts).unwrap();
    init_git_repo(&repo_path);
    repo_path
}

fn create_py_repo(parent: &Path) -> PathBuf {
    let repo_path = parent.join("py_repo");
    fs::create_dir_all(&repo_path).unwrap();

    let app_py = r#"class Processor:
    def __init__(self):
        self.data = []

    def process(self, item):
        # Process item
        print(f"Processing {item}")
        return True

def main():
    p = Processor()
    p.process("test")
"#;
    fs::write(repo_path.join("app.py"), app_py).unwrap();
    init_git_repo(&repo_path);
    repo_path
}

fn create_go_repo(parent: &Path) -> PathBuf {
    let repo_path = parent.join("go_repo");
    fs::create_dir_all(&repo_path).unwrap();

    let main_go = r#"package main

import "fmt"

type Server struct {
	Port int
}

func (s *Server) Start() {
	// Start server
	fmt.Println("Starting...")
}

func main() {
	s := &Server{Port: 8080}
	s.Start()
}
"#;
    fs::write(repo_path.join("main.go"), main_go).unwrap();
    init_git_repo(&repo_path);
    repo_path
}

fn create_mixed_repo(parent: &Path) -> PathBuf {
    let repo_path = parent.join("mixed_repo");
    fs::create_dir_all(&repo_path).unwrap();

    fs::write(repo_path.join("secret.env"), "Secret=API_KEY_12345678901234567890123456789012").unwrap();
    fs::write(repo_path.join("normal.txt"), "Normal file").unwrap();
    fs::write(repo_path.join("ignore_me.log"), "Ignored file").unwrap();
    fs::write(repo_path.join(".gitignore"), "*.log\nsecret.env").unwrap();

    init_git_repo(&repo_path);
    repo_path
}

// --- Tests ---

#[test]
fn test_basic_xml_output_rust() {
    let temp = TempDir::new().unwrap();
    let repo_path = create_rust_repo(temp.path());
    let output_path = temp.path().join("output_1.xml");

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_rustymix"));
    cmd.arg(repo_path.to_str().unwrap())
       .arg("-o")
       .arg(output_path.to_str().unwrap())
       .assert()
       .success();

    let content = fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("<repomix>"));
    assert!(content.contains("src/main.rs"));
}

#[test]
fn test_markdown_output_ts() {
    let temp = TempDir::new().unwrap();
    let repo_path = create_ts_repo(temp.path());
    let output_path = temp.path().join("output_2.md");

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_rustymix"));
    cmd.arg(repo_path.to_str().unwrap())
       .arg("--style")
       .arg("markdown")
       .arg("-o")
       .arg(output_path.to_str().unwrap())
       .assert()
       .success();

    let content = fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("# File Summary"));
    assert!(content.contains("class UserManager"));
}

#[test]
fn test_compression_python() {
    let temp = TempDir::new().unwrap();
    let repo_path = create_py_repo(temp.path());
    let output_path = temp.path().join("output_3.txt");

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_rustymix"));
    cmd.arg(repo_path.to_str().unwrap())
       .arg("--compress")
       .arg("-o")
       .arg(output_path.to_str().unwrap())
       .assert()
       .success();

    let content = fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("class Processor"));
    // Compression usually keeps signatures.
}

#[test]
fn test_security_check() {
    let temp = TempDir::new().unwrap();
    let repo_path = create_mixed_repo(temp.path());

    // Create a leaked token file that is tracked by git
    // We add it AFTER git init/add in helper, so we need to add it manually here
    let leaked_path = repo_path.join("leaked_token.txt");
    fs::write(&leaked_path, "token = 'ghp_ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890'").unwrap();

    // We intentionally don't add it to gitignore so it would be picked up

    let output_path = temp.path().join("output_4.xml");

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_rustymix"));
    cmd.arg(repo_path.to_str().unwrap())
       .arg("--no-gitignore") // To ensure we traverse everything not in .gitignore
       .arg("-o")
       .arg(output_path.to_str().unwrap())
       .assert()
       .success();

    let content = fs::read_to_string(&output_path).unwrap();
    // It should NOT contain leaked_token.txt content because security check is on by default
    assert!(!content.contains("leaked_token.txt"), "Security check failed, file included");
}

#[test]
fn test_git_logs_go() {
    let temp = TempDir::new().unwrap();
    let repo_path = create_go_repo(temp.path());
    let output_path = temp.path().join("output_5.xml");

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_rustymix"));
    cmd.arg(repo_path.to_str().unwrap())
       .arg("--include-logs")
       .arg("-o")
       .arg(output_path.to_str().unwrap())
       .assert()
       .success();

    let content = fs::read_to_string(&output_path).unwrap();
    assert!(content.contains("<git_log>"));
}

#[test]
fn test_remove_comments_rust() {
    let temp = TempDir::new().unwrap();
    let repo_path = create_rust_repo(temp.path());
    let output_path = temp.path().join("output_6.xml");

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_rustymix"));
    cmd.arg(repo_path.to_str().unwrap())
       .arg("--remove-comments")
       .arg("-o")
       .arg(output_path.to_str().unwrap())
       .assert()
       .success();

    let content = fs::read_to_string(&output_path).unwrap();
    assert!(!content.contains("This is a comment"));
}

#[test]
fn test_ignore_patterns() {
    let temp = TempDir::new().unwrap();
    let repo_path = create_mixed_repo(temp.path());
    let output_path = temp.path().join("output_7.xml");

    // ignore_me.log is in .gitignore
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_rustymix"));
    cmd.arg(repo_path.to_str().unwrap())
       .arg("-o")
       .arg(output_path.to_str().unwrap())
       .assert()
       .success();

    let content = fs::read_to_string(&output_path).unwrap();
    assert!(!content.contains("ignore_me.log"));
}

#[test]
fn test_cli_ignore() {
    let temp = TempDir::new().unwrap();
    let repo_path = create_ts_repo(temp.path());
    let output_path = temp.path().join("output_7b.xml");

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_rustymix"));
    cmd.arg(repo_path.to_str().unwrap())
       .arg("--ignore")
       .arg("src/index.ts")
       .arg("-o")
       .arg(output_path.to_str().unwrap())
       .assert()
       .success();

    // File should be created but empty (or valid empty XML) or just not contain index.ts
    if output_path.exists() {
        let content = fs::read_to_string(&output_path).unwrap();
        assert!(!content.contains("index.ts"));
    }
}

#[test]
fn test_config_file() {
    let temp = TempDir::new().unwrap();
    let repo_path = create_ts_repo(temp.path());
    let config_path = temp.path().join("custom_config.json");
    let output_path = temp.path().join("output_8.json");

    let config_content = r#"{
  "output": {
    "style": "json",
    "topFilesLength": 10
  },
  "ignore": {
    "customPatterns": ["**/*.ts"]
  },
  "security": {
    "enableSecurityCheck": true
  }
}"#;
    fs::write(&config_path, config_content).unwrap();

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_rustymix"));
    cmd.arg(repo_path.to_str().unwrap())
       .arg("--config")
       .arg(config_path.to_str().unwrap())
       .arg("-o")
       .arg(output_path.to_str().unwrap())
       .assert()
       .success();

    if output_path.exists() {
        let content = fs::read_to_string(&output_path).unwrap();
        // Since we ignore **/*.ts, and the repo only has ts files, result should not have index.ts
        assert!(!content.contains("index.ts"));
    }
}

// --- New Bulk Processing Tests ---

#[test]
fn test_bulk_intent_processing() {
    let temp = TempDir::new().unwrap();
    let repo_path = create_rust_repo(temp.path());

    // Create intent directory with multiple intent files
    let intent_dir = temp.path().join("intents");
    fs::create_dir_all(&intent_dir).unwrap();

    fs::write(intent_dir.join("fix_bug.txt"), "Fix the bug in the main function.").unwrap();
    fs::write(intent_dir.join("add_feature.txt"), "Add a new feature to TestStruct.").unwrap();

    // Run rustymix with --intent pointed to the directory
    // Note: When using bulk intent, if output path is not specified, it defaults to current dir?
    // Or if output is specified, it uses it as base?
    // Let's specify output as a directory to be safe and clean
    let output_dir = temp.path().join("results");
    fs::create_dir_all(&output_dir).unwrap();

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_rustymix"));
    cmd.arg(repo_path.to_str().unwrap())
       .arg("--intent")
       .arg(intent_dir.to_str().unwrap())
       .arg("-o")
       .arg(output_dir.to_str().unwrap())
       .arg("--style")
       .arg("markdown") // Easier to check content
       .assert()
       .success();

    // Verify output files exist
    // Convention: repomix-{intent_name}.{ext}
    let fix_bug_output = output_dir.join("repomix-fix_bug.md");
    let add_feature_output = output_dir.join("repomix-add_feature.md");

    assert!(fix_bug_output.exists(), "fix_bug output file missing");
    assert!(add_feature_output.exists(), "add_feature output file missing");

    // Verify content injection
    let fix_bug_content = fs::read_to_string(fix_bug_output).unwrap();
    assert!(fix_bug_content.contains("THE USER WANTS TO: Fix the bug in the main function."), "Intent not found in fix_bug output");
    assert!(fix_bug_content.contains("Attached is the SKELETON of the codebase."), "Header instructions missing");

    let add_feature_content = fs::read_to_string(add_feature_output).unwrap();
    assert!(add_feature_content.contains("THE USER WANTS TO: Add a new feature to TestStruct."), "Intent not found in add_feature output");
}

#[test]
fn test_bulk_intent_processing_xml() {
    let temp = TempDir::new().unwrap();
    let repo_path = create_rust_repo(temp.path());

    let intent_dir = temp.path().join("intents_xml");
    fs::create_dir_all(&intent_dir).unwrap();
    fs::write(intent_dir.join("task1.txt"), "Task 1").unwrap();

    let output_dir = temp.path().join("results_xml");
    fs::create_dir_all(&output_dir).unwrap();

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_rustymix"));
    cmd.arg(repo_path.to_str().unwrap())
       .arg("--intent")
       .arg(intent_dir.to_str().unwrap())
       .arg("-o")
       .arg(output_dir.to_str().unwrap())
       .arg("--style")
       .arg("xml")
       .assert()
       .success();

    let task1_output = output_dir.join("repomix-task1.xml");
    assert!(task1_output.exists());

    let content = fs::read_to_string(task1_output).unwrap();
    assert!(content.contains("THE USER WANTS TO: Task 1"));
    assert!(content.contains("<repomix>"));
}
