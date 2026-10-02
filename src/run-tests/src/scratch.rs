// MODE: DEV
// PACKAGE: PROD

//! The per-run scratch root, marker-based cleanup of scratch test roots
//! elsewhere under /tmp, and a real signal handler so cleanup still runs on
//! SIGINT/SIGTERM -- a bare Drop guard alone does not fire on signal
//! termination.

use std::fs;
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::sync::atomic::{AtomicI32, Ordering};
#[cfg(unix)]
use std::sync::OnceLock;

// The signal machinery is unix-only, like the handler that uses it: there is
// no SIGINT/SIGTERM delivery to hook on Windows, and unused statics fail the
// build there under -D warnings.
#[cfg(unix)]
static SIGNAL_PIPE_WRITE: OnceLock<i32> = OnceLock::new();
#[cfg(unix)]
static RECEIVED_SIGNAL: AtomicI32 = AtomicI32::new(0);

#[cfg(unix)]
extern "C" fn handle_signal(sig: libc::c_int) {
    RECEIVED_SIGNAL.store(sig, Ordering::SeqCst);
    if let Some(&fd) = SIGNAL_PIPE_WRITE.get() {
        let byte = [1u8];
        unsafe {
            libc::write(fd, byte.as_ptr() as *const libc::c_void, 1);
        }
    }
}

/// Installs a SIGINT/SIGTERM handler that runs `cleanup` on a background
/// thread (never inside the signal handler itself, which only performs the
/// one async-signal-safe `write()` to a self-pipe) and then exits with the
/// conventional 128+signal code.
#[cfg(unix)]
pub fn install_signal_cleanup(cleanup: impl FnOnce() + Send + 'static) {
    let mut fds = [0i32; 2];
    if unsafe { libc::pipe(fds.as_mut_ptr()) } != 0 {
        return;
    }
    let (read_fd, write_fd) = (fds[0], fds[1]);
    if SIGNAL_PIPE_WRITE.set(write_fd).is_err() {
        return;
    }
    unsafe {
        libc::signal(
            libc::SIGTERM,
            handle_signal as *const () as libc::sighandler_t,
        );
        libc::signal(
            libc::SIGINT,
            handle_signal as *const () as libc::sighandler_t,
        );
    }
    std::thread::spawn(move || {
        let mut buf = [0u8; 1];
        let n = unsafe { libc::read(read_fd, buf.as_mut_ptr() as *mut libc::c_void, 1) };
        if n > 0 {
            cleanup();
            let sig = RECEIVED_SIGNAL.load(Ordering::SeqCst);
            std::process::exit(128 + sig);
        }
    });
}

#[cfg(windows)]
mod console {
    use std::sync::Mutex;

    type Cleanup = Box<dyn FnOnce() + Send>;
    pub static CLEANUP: Mutex<Option<Cleanup>> = Mutex::new(None);

    extern "system" {
        pub fn SetConsoleCtrlHandler(
            handler: Option<unsafe extern "system" fn(u32) -> i32>,
            add: i32,
        ) -> i32;
    }

    /// Ctrl-C and Ctrl-Break (and a closing console window). Windows runs this
    /// on a thread of its own, so it may do the cleanup itself before ending
    /// the process with the code a shell reports for SIGINT.
    pub unsafe extern "system" fn on_ctrl(_kind: u32) -> i32 {
        let cleanup = CLEANUP.lock().ok().and_then(|mut slot| slot.take());
        if let Some(cleanup) = cleanup {
            cleanup();
        }
        std::process::exit(130)
    }
}

/// The Windows counterpart of the signal cleanup above: a console control
/// handler that removes the lock and the scratch root on Ctrl-C.
#[cfg(windows)]
pub fn install_signal_cleanup(cleanup: impl FnOnce() + Send + 'static) {
    if let Ok(mut slot) = console::CLEANUP.lock() {
        *slot = Some(Box::new(cleanup));
    }
    unsafe {
        console::SetConsoleCtrlHandler(Some(console::on_ctrl), 1);
    }
}

#[cfg(not(any(unix, windows)))]
pub fn install_signal_cleanup(_cleanup: impl FnOnce() + Send + 'static) {}

pub struct ScratchRoot {
    pub path: PathBuf,
    pub run_id: String,
    repo_root: PathBuf,
}

impl ScratchRoot {
    pub fn create(
        base: &Path,
        repo_root: &Path,
        pid: u32,
        unix_time: u64,
    ) -> std::io::Result<Self> {
        let template = base.join(format!("ai-skills-tests.{pid}-{unix_time}"));
        fs::create_dir_all(&template)?;
        Ok(ScratchRoot {
            path: template,
            run_id: format!("run-tests.{pid}.{unix_time}"),
            repo_root: repo_root.to_path_buf(),
        })
    }
}

/// Removes repo_root/benchmark/results/*/.staging, matching the original's
/// own end-of-run staging cleanup.
pub fn remove_benchmark_staging(repo_root: &Path) {
    let Ok(entries) = fs::read_dir(repo_root.join("benchmark/results")) else {
        return;
    };
    for entry in entries.flatten() {
        let staging = entry.path().join(".staging");
        let _ = fs::remove_dir_all(staging);
    }
}

impl Drop for ScratchRoot {
    fn drop(&mut self) {
        cleanup_marked_test_roots(&self.path, &self.run_id);
        let _ = fs::remove_dir_all(&self.path);
        remove_benchmark_staging(&self.repo_root);
    }
}

/// Scans `/tmp` and `tmp_dir` at maxdepth 1 for directories matching the
/// literal glob `t.?????` (exactly five characters after "t.", the width
/// lib-test.sh's own mktemp template produces), removing only the ones
/// carrying THIS run's own `.ai-skills-test-run-id` marker.
pub fn cleanup_marked_test_roots(tmp_dir: &Path, run_id: &str) {
    let system_tmp = crate::platform::system_tmp();
    for scan in [system_tmp.as_path(), tmp_dir] {
        let Ok(entries) = fs::read_dir(scan) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if !is_scratch_root_name(name) {
                continue;
            }
            let marker = path.join(".ai-skills-test-run-id");
            let Ok(marker_value) = fs::read_to_string(&marker) else {
                continue;
            };
            if marker_value.lines().next() == Some(run_id) {
                let _ = fs::remove_dir_all(&path);
            }
        }
    }
}

/// `t.?????` -- literally "t." followed by exactly five arbitrary characters.
fn is_scratch_root_name(name: &str) -> bool {
    name.len() == 7 && name.starts_with("t.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scratch_root_name_matches_exactly_five_trailing_characters() {
        assert!(is_scratch_root_name("t.abcde"));
        assert!(!is_scratch_root_name("t.abcd"));
        assert!(!is_scratch_root_name("t.abcdef"));
        assert!(!is_scratch_root_name("x.abcde"));
    }

    #[test]
    fn cleanup_removes_only_the_matching_run_id() {
        let base =
            std::env::temp_dir().join(format!("run-tests-scratch-test-{}", std::process::id()));
        fs::create_dir_all(&base).unwrap();
        let ours = base.join("t.aaaaa");
        let theirs = base.join("t.bbbbb");
        fs::create_dir_all(&ours).unwrap();
        fs::create_dir_all(&theirs).unwrap();
        fs::write(ours.join(".ai-skills-test-run-id"), "run-tests.1.111\n").unwrap();
        fs::write(theirs.join(".ai-skills-test-run-id"), "run-tests.2.222\n").unwrap();

        cleanup_marked_test_roots(&base, "run-tests.1.111");

        assert!(!ours.exists());
        assert!(theirs.exists());
        let _ = fs::remove_dir_all(&base);
    }
}
