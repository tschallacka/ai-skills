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
/// live".
fn live_owner(parent: &Path) -> Option<i32> {
    let contents = fs::read_to_string(parent.join("harness.pid")).ok()?;
    let pid: i32 = contents.trim().parse().ok()?;
    if is_live(pid) {
        Some(pid)
    } else {
        None
    }
}

#[cfg(unix)]
fn is_live(pid: i32) -> bool {
    unsafe { libc::kill(pid, 0) == 0 }
}

/// Windows has no `kill(pid, 0)`. The equivalent question -- is a process
/// with this pid still running -- is answered by opening it for a query and
/// asking for its exit code: a running process reports STILL_ACTIVE. A pid
/// that cannot be opened (gone, or another user's) is not live, the same
/// answer `kill` gives for ESRCH and EPERM alike.
#[cfg(windows)]
fn is_live(pid: i32) -> bool {
    use std::ffi::c_void;

    extern "system" {
        fn OpenProcess(access: u32, inherit_handle: i32, pid: u32) -> *mut c_void;
        fn GetExitCodeProcess(process: *mut c_void, exit_code: *mut u32) -> i32;
        fn CloseHandle(handle: *mut c_void) -> i32;
    }
    const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
    const STILL_ACTIVE: u32 = 259;

    if pid <= 0 {
        return false;
    }
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid as u32);
        if process.is_null() {
            return false;
        }
        let mut exit_code = 0u32;
        let known = GetExitCodeProcess(process, &mut exit_code);
        CloseHandle(process);
        known != 0 && exit_code == STILL_ACTIVE
    }
}

#[cfg(not(any(unix, windows)))]
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
        // A child that exits at once, by whatever the platform calls that.
        let mut child = if cfg!(windows) {
            let mut c = Command::new("cmd");
            c.args(["/C", "exit 0"]);
            c
        } else {
            Command::new("true")
        }
        .spawn()
        .unwrap();
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
