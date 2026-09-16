// MODE: DEV
// PACKAGE: PROD

//! The machine-wide, noclobber-equivalent lock: two suite runs on this
//! machine collide over the cargo target dir, the chat beacon port, and the
//! short /tmp test roots, so only one may run at a time unless the caller
//! explicitly opts out.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

pub const LOCK_PATH: &str = "/tmp/ai-skills-run-tests.lock";
pub const LOCK_MARKER: &str = "ai-skills-run-tests";

pub enum AcquireResult {
    Acquired,
    Bypassed,
    RefusedLiveHolder {
        pid: String,
        started: String,
        in_dir: String,
        command: String,
    },
    RefusedLostRace {
        pid: String,
    },
}

fn write_lock(path: &Path, repo_root: &Path) -> std::io::Result<()> {
    let mut file = OpenOptions::new().write(true).create_new(true).open(path)?;
    let now = chrono_like_utc_now();
    write!(
        file,
        "{}\n{}\n{}\n{}\n",
        std::process::id(),
        LOCK_MARKER,
        repo_root.display(),
        now
    )
}

/// A minimal UTC "YYYY-MM-DDTHH:MM:SSZ" stamp via `date -u`, matching the
/// bash original's own `$(date -u +%Y-%m-%dT%H:%M:%SZ)` exactly rather than
/// pulling in a date/time crate for one timestamp.
fn chrono_like_utc_now() -> String {
    Command::new("date")
        .args(["-u", "+%Y-%m-%dT%H:%M:%SZ"])
        .output()
        .ok()
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .unwrap_or_default()
}

fn read_lock_lines(path: &Path) -> Vec<String> {
    std::fs::read_to_string(path)
        .map(|content| content.lines().map(str::to_string).collect())
        .unwrap_or_default()
}

fn lock_holder_command(pid: &str) -> String {
    for flag in ["args=", "command="] {
        if let Ok(out) = Command::new("ps").args(["-p", pid, "-o", flag]).output() {
            let text = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !text.is_empty() {
                return text;
            }
        }
    }
    String::new()
}

fn lock_holder_is_live(pid: &str) -> bool {
    if pid.is_empty() || !pid.bytes().all(|b| b.is_ascii_digit()) {
        return false;
    }
    let command = lock_holder_command(pid);
    // The bash original only ever needs to recognize another bash
    // invocation ("bash .../run-tests.sh", matched via the literal
    // substring "run-tests.sh"). Once a compiled binary can hold this same
    // lock, its own `ps` command line is just "run-tests" -- no ".sh" --
    // so checking only for "run-tests.sh" would make a live compiled-binary
    // holder look stale to itself, defeating the whole mutex the moment two
    // compiled-binary runs (or a bash and a compiled-binary run) contend
    // for it. "run-tests" alone matches both shapes and is not a
    // meaningfully weaker check: no unrelated process's own command line
    // plausibly contains it by coincidence.
    !command.is_empty() && command.contains("run-tests")
}

pub struct Lock {
    path: PathBuf,
    held: bool,
    pid: u32,
}

impl Lock {
    /// Acquires the machine-wide lock, following the original's own retry
    /// shape: a failed create reads the existing holder's pid, refuses if
    /// it is live, otherwise reclaims a stale lock with one retry.
    /// `lock_path` is injectable so tests never contend for the real,
    /// machine-wide LOCK_PATH.
    pub fn acquire(
        lock_path: &Path,
        repo_root: &Path,
        allow_concurrent: bool,
    ) -> (Self, AcquireResult) {
        let path = lock_path.to_path_buf();
        let mut lock = Lock {
            path: path.clone(),
            held: false,
            pid: std::process::id(),
        };
        if allow_concurrent {
            return (lock, AcquireResult::Bypassed);
        }
        if write_lock(&path, repo_root).is_ok() {
            lock.held = true;
            return (lock, AcquireResult::Acquired);
        }
        let existing = read_lock_lines(&path);
        let held_pid = existing.first().cloned().unwrap_or_default();
        if lock_holder_is_live(&held_pid) {
            return (
                lock,
                AcquireResult::RefusedLiveHolder {
                    started: existing.get(3).cloned().unwrap_or_default(),
                    in_dir: existing.get(2).cloned().unwrap_or_default(),
                    command: lock_holder_command(&held_pid),
                    pid: held_pid,
                },
            );
        }
        let _ = std::fs::remove_file(&path);
        if write_lock(&path, repo_root).is_ok() {
            lock.held = true;
            return (lock, AcquireResult::Acquired);
        }
        let existing = read_lock_lines(&path);
        (
            lock,
            AcquireResult::RefusedLostRace {
                pid: existing
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "unknown".to_string()),
            },
        )
    }

    /// Only ever removes a lock this process itself owns: a run that
    /// refused to start must not delete a lock legitimately held elsewhere.
    pub fn release(&mut self) {
        if !self.held {
            return;
        }
        let owner = read_lock_lines(&self.path)
            .first()
            .cloned()
            .unwrap_or_default();
        if owner == self.pid.to_string() {
            let _ = std::fs::remove_file(&self.path);
        }
        self.held = false;
    }
}

impl Drop for Lock {
    fn drop(&mut self) {
        self.release();
    }
}

/// Stateless equivalent of `Lock::release`, for the signal-handler cleanup
/// path where no `Lock` value can be safely shared across threads: only
/// ever removes a lock this process itself owns.
pub fn release_lock_by_path(path: &Path, pid: u32) {
    let owner = read_lock_lines(path).first().cloned().unwrap_or_default();
    if owner == pid.to_string() {
        let _ = std::fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_pid_string_with_non_digits_is_never_live() {
        assert!(!lock_holder_is_live("abc"));
        assert!(!lock_holder_is_live(""));
    }
}
