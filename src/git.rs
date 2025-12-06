use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;
use std::process::Command;

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
