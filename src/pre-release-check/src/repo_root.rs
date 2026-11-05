// MODE: DEV
// PACKAGE: PROD

//! Repository-root discovery and the one git read every gate shares: the
//! full tracked-file list. Mirrors pre-push-check's own simple
//! `git rev-parse --show-toplevel` discovery -- this tool, like that one,
//! only ever runs against a real checkout, never needs
//! `PLANNING_SKILL_ROOT`.

use std::path::PathBuf;
use std::process::Command;

pub fn discover_repo_root() -> Option<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!text.is_empty()).then(|| PathBuf::from(text))
}

/// Every tracked path in the repository, repo-root-relative. Empty on any
/// git failure rather than erroring: a gate reading this treats "found
/// nothing" and "git failed" the same way, since either means it has
/// nothing sound to report on.
pub fn tracked_files(repo_root: &std::path::Path) -> Vec<String> {
    let output = Command::new("git")
        .args(["ls-files"])
        .current_dir(repo_root)
        .output();
    match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

/// Every tracked path under `dir` (repo-root-relative, `dir` itself
/// repo-root-relative with no trailing slash).
pub fn tracked_files_under(repo_root: &std::path::Path, dir: &str) -> Vec<String> {
    let output = Command::new("git")
        .args(["ls-files", dir])
        .current_dir(repo_root)
        .output();
    match output {
        Ok(out) if out.status.success() => String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(str::to_string)
            .collect(),
        _ => Vec::new(),
    }
}
