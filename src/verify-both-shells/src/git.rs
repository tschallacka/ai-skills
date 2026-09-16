// MODE: DEV
// PACKAGE: PROD
use std::path::Path;
use std::process::Command;

pub fn worktree_add(src: &Path, wt: &Path) -> bool {
    Command::new("git")
        .args(["-C"])
        .arg(src)
        .args(["worktree", "add", "--detach", "--quiet"])
        .arg(wt)
        .arg("HEAD")
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

pub fn worktree_remove(src: &Path, wt: &Path) {
    let _ = Command::new("git")
        .args(["-C"])
        .arg(src)
        .args(["worktree", "remove", "--force"])
        .arg(wt)
        .output();
}

/// Every path from `git worktree list --porcelain`'s own `worktree <path>`
/// lines.
pub fn worktree_paths(src: &Path) -> Vec<String> {
    let output = Command::new("git")
        .args(["-C"])
        .arg(src)
        .args(["worktree", "list", "--porcelain"])
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.strip_prefix("worktree "))
        .map(|path| path.to_string())
        .collect()
}

pub fn diff_name_only(src: &Path) -> Vec<String> {
    let output = Command::new("git")
        .args(["-C"])
        .arg(src)
        .args(["diff", "HEAD", "--name-only"])
        .output();
    let Ok(output) = output else {
        return Vec::new();
    };
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(|l| l.to_string())
        .collect()
}

pub fn rev_parse_short_head(src: &Path) -> String {
    let output = Command::new("git")
        .args(["-C"])
        .arg(src)
        .args(["rev-parse", "--short", "HEAD"])
        .output();
    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).trim().to_string(),
        _ => String::new(),
    }
}
