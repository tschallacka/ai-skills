// MODE: DEV
// PACKAGE: PROD
use std::path::Path;
use std::process::Command;

pub fn discover_repo_root() -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
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

pub fn status_porcelain(repo_root: &Path) -> String {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(repo_root)
        .output()
        .expect("git status --porcelain failed to spawn");
    String::from_utf8_lossy(&output.stdout).to_string()
}

/// `git ls-files --error-unmatch <path>` succeeding means the path is already
/// tracked -- only a genuinely new file can be missing a manifest row.
pub fn is_tracked(repo_root: &Path, path: &str) -> bool {
    Command::new("git")
        .args(["ls-files", "--error-unmatch", path])
        .current_dir(repo_root)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn rev_parse_verify_quiet(repo_root: &Path, reference: &str) -> bool {
    Command::new("git")
        .args(["rev-parse", "--verify", "--quiet", reference])
        .current_dir(repo_root)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

pub fn merge_base(repo_root: &Path, base: &str) -> Option<String> {
    let output = Command::new("git")
        .args(["merge-base", "HEAD", base])
        .current_dir(repo_root)
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

/// Count of non-blank lines of `git log --oneline <range> -- <path>`,
/// matching bash's own `grep -c .`.
pub fn log_oneline_count(repo_root: &Path, range: &str, path: &str) -> usize {
    let output = Command::new("git")
        .args(["log", "--oneline", range, "--", path])
        .current_dir(repo_root)
        .output()
        .expect("git log --oneline failed to spawn");
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .count()
}
