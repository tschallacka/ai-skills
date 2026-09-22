// MODE: DEV
// PACKAGE: PROD
//! Repository-root discovery: `PLANNING_SKILL_ROOT` first (set by
//! `plan_exec_compiled_binary_if_present` on every wired invocation),
//! `current_exe()`-anchored ancestor search as the fallback for a standalone
//! invocation (tests, or running the binary directly).

use std::env;
use std::path::{Path, PathBuf};

/// Never fails: on a discovery failure, returns a sentinel path that does
/// not exist, so the caller's own ordinary `repo_root.is_dir()` reachability
/// check (run AFTER the `--push-to` short-circuit) is what reports the
/// failure, rather than main() reporting it out of order before `--push-to`
/// is even considered.
pub fn discover_repo_root_or_sentinel() -> PathBuf {
    discover_repo_root().unwrap_or_else(|_| PathBuf::from("/nonexistent-ci-test-scope-repo-root"))
}

fn discover_repo_root() -> Result<PathBuf, String> {
    if let Ok(root) = env::var("PLANNING_SKILL_ROOT") {
        if !root.is_empty() {
            return Ok(PathBuf::from(root));
        }
    }
    let self_path = env::current_exe().unwrap_or_else(|_| PathBuf::from("ci-test-scope"));
    let mut dir = self_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    loop {
        if dir.join("planning/scripts").is_dir() {
            return Ok(dir);
        }
        let Some(parent) = dir.parent() else {
            return Err(format!(
                "could not locate the repository root from {}",
                self_path.display()
            ));
        };
        dir = parent.to_path_buf();
    }
}
