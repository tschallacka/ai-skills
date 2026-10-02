// MODE: DEV
// PACKAGE: PROD
//! Real `git` subprocess helpers, run with `repo_root` as the working
//! directory -- identical shape to ci-scope's own git.rs (goal 21), since
//! ci-test-scope.sh's own change-set-gathering git plumbing is byte-for-byte
//! the same as ci-scope.sh's.

use std::path::Path;
use std::process::{Command, Stdio};

pub fn git_dir_exists(repo_root: &Path) -> bool {
    Command::new("git")
        .arg("rev-parse")
        .arg("--git-dir")
        .current_dir(repo_root)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn ref_resolves(repo_root: &Path, reference: &str) -> bool {
    Command::new("git")
        .arg("rev-parse")
        .arg("--verify")
        .arg(reference)
        .current_dir(repo_root)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

pub fn merge_base(repo_root: &Path, base: &str) -> Option<String> {
    let output = Command::new("git")
        .arg("merge-base")
        .arg(base)
        .arg("HEAD")
        .current_dir(repo_root)
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if text.is_empty() {
        None
    } else {
        Some(text)
    }
}

pub fn diff_name_only(repo_root: &Path, merge_base: &str) -> Option<String> {
    let output = Command::new("git")
        .arg("diff")
        .arg("--name-only")
        .arg(format!("{merge_base}..HEAD"))
        .current_dir(repo_root)
        .stderr(Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&output.stdout).to_string())
}
