// MODE: DEV
// PACKAGE: PROD
use crate::{git, worktree};
use std::fs;
use std::path::Path;

/// AR-79: sweeping removes BOTH the worktree itself (`git worktree remove
/// --force`) AND its own parent directory (the mktemp-created directory that
/// holds `harness.pid`) -- `git worktree remove` alone never touches that
/// parent.
pub fn remove_worktree_and_parent(src: &Path, wt: &Path) {
    git::worktree_remove(src, wt);
    if let Some(parent) = wt.parent() {
        let _ = fs::remove_dir_all(parent);
    }
}

/// Sweeps every OTHER stale-looking worktree `git worktree list --porcelain`
/// reports, leaving `own_wt` and any worktree with a live owner untouched.
pub fn sweep_stale_worktrees(src: &Path, own_wt: &Path) {
    for path_str in git::worktree_paths(src) {
        if !worktree::is_stale_candidate(&path_str) {
            continue;
        }
        let path = std::path::PathBuf::from(&path_str);
        if path == own_wt {
            continue;
        }
        let Some(parent) = path.parent() else {
            continue;
        };
        if let Some(owner) = live_owner(parent) {
            println!(
                "leaving live worktree {} (harness pid {owner})",
                path.display()
            );
            continue;
        }
        println!("sweeping leftover worktree {}", path.display());
        remove_worktree_and_parent(src, &path);
    }
}

/// `Some(pid)` when `<parent>/harness.pid` names a pid that is still
/// signalable (`kill -0`-equivalent). AR-80: a missing file, an empty
/// value, or a non-numeric value are ALL treated identically as "not
/// live" -- matching bash's own `cat ... 2>/dev/null || true` plus
/// `[ -n "$owner" ]` guard exactly.
fn live_owner(parent: &Path) -> Option<i32> {
    let contents = fs::read_to_string(parent.join("harness.pid")).ok()?;
    let pid: i32 = contents.trim().parse().ok()?;
    if is_live(pid) {
        Some(pid)
    } else {
        None
    }
}

// This script's own verify-both-shells.sh has no meaningful bash-comparison
// workflow on Windows (no bash to compare), but the crate still has to
// compile there since ci-subjects.sh's own planning_commands catch-all
// builds every workspace member on every platform. `kill(pid, 0)` is POSIX
// only; there is no signalable-pid check on offer here, so a non-unix build
// always answers "not live" -- the same safe default a missing/malformed
// harness.pid already gets above.
#[cfg(unix)]
fn is_live(pid: i32) -> bool {
    unsafe { libc::kill(pid, 0) == 0 }
}

#[cfg(not(unix))]
fn is_live(_pid: i32) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn scratch_dir(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "verify-both-shells-sweep-test-{tag}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_pid_naming_this_test_process_itself_is_live() {
        let dir = scratch_dir("live");
        fs::write(dir.join("harness.pid"), std::process::id().to_string()).unwrap();
        assert_eq!(live_owner(&dir), Some(std::process::id() as i32));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_pid_that_does_not_exist_is_dead() {
        let dir = scratch_dir("dead-pid");
        // Spawn and wait on a short-lived child so its pid is (almost
        // certainly) no longer live by the time we check it.
        let mut child = Command::new("true").spawn().unwrap();
        let dead_pid = child.id() as i32;
        let _ = child.wait();
        fs::write(dir.join("harness.pid"), dead_pid.to_string()).unwrap();
        assert_eq!(live_owner(&dir), None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_pid_file_is_dead() {
        let dir = scratch_dir("missing");
        assert_eq!(live_owner(&dir), None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_empty_pid_file_is_dead() {
        let dir = scratch_dir("empty");
        fs::write(dir.join("harness.pid"), "").unwrap();
        assert_eq!(live_owner(&dir), None);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_non_numeric_pid_file_is_dead() {
        let dir = scratch_dir("non-numeric");
        fs::write(dir.join("harness.pid"), "not-a-pid\n").unwrap();
        assert_eq!(live_owner(&dir), None);
        let _ = fs::remove_dir_all(&dir);
    }
}
