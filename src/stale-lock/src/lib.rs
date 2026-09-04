// MODE: DEV
// PACKAGE: PROD
//! An advisory file lock for coordinating separate processes over a shared
//! resource (a session registry, a per-file "a server is starting" marker):
//! `create_new` on a lock file is the mutual exclusion, and a modification-age
//! check is the recovery when the holder crashed instead of releasing it.
//!
//! This does not replace a real IPC primitive when contention is high or the
//! critical section is long; it exists for the shape this repo keeps
//! needing: several short-lived CLI invocations racing to do a small,
//! idempotent-once-done piece of setup (write a registry entry, start a
//! server) without two of them doing it at once.

use std::fs;
use std::io;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

/// Held for as long as the lock is wanted; the file is removed on drop, or by
/// the next waiter once it has been unmodified for longer than that waiter's
/// own `stale_after`. Release does not depend on `Drop` running at all —
/// reclaiming an abandoned lock is entirely `mtime`-based in `acquire`'s
/// retry loop, so a holder that was killed or crashed (no unwind, no `Drop`)
/// wedges nothing forever: the next waiter reclaims it once it is older than
/// `stale_after`, the same as a holder that dropped normally just never runs.
pub struct StaleLock {
    path: PathBuf,
    /// Written into the file at acquire time and checked again at drop time,
    /// so a holder whose lock was reclaimed out from under it (its own
    /// `stale_after` elapsed while it was still legitimately working) does
    /// not then delete the *new* holder's file when it finally drops —
    /// without this, `Drop` would unconditionally remove whatever is at
    /// `path`, which after a steal is someone else's lock, not this one's.
    token: String,
}

fn fresh_token() -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    format!(
        "{}-{}-{}",
        std::process::id(),
        now.as_nanos(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    )
}

impl StaleLock {
    /// Acquire the lock at `path`, retrying for a few seconds while another
    /// process holds it. A held lock file untouched for longer than
    /// `stale_after` is assumed to belong to a process that no longer exists
    /// and is removed so the retry can proceed. Different callers may name
    /// different `stale_after` values for the same path; the file itself
    /// carries no expectation of one.
    pub fn acquire(path: &Path, stale_after: Duration) -> io::Result<Self> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let token = fresh_token();
        for _ in 0..500 {
            match fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(path)
            {
                Ok(mut file) => {
                    // Best-effort: if the write fails partway, the lock is
                    // still held (create_new is the actual exclusion) — this
                    // token only affects who is allowed to remove it later.
                    let _ = file.write_all(token.as_bytes());
                    return Ok(Self {
                        path: path.to_path_buf(),
                        token,
                    });
                }
                Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {
                    // metadata() can itself fail here if the file vanished
                    // between the create_new above and this call (its
                    // holder just dropped it, or a racing waiter just stole
                    // it) — Ok(_) is required before touching .modified(),
                    // so that race simply falls through to the sleep/retry
                    // below rather than erroring.
                    if let Ok(metadata) = fs::metadata(path) {
                        if metadata
                            .modified()
                            .ok()
                            .and_then(|modified| modified.elapsed().ok())
                            .is_some_and(|age| age > stale_after)
                        {
                            // Discarded deliberately: removal can lose a
                            // race against another waiter's own steal, or
                            // against the original holder finally dropping
                            // it — either way the file is gone, which is the
                            // outcome this call wanted, so an error here
                            // (already-gone included) is not this call's to
                            // report.
                            let _ = fs::remove_file(path);
                        }
                    }
                    std::thread::sleep(Duration::from_millis(10));
                }
                Err(error) => return Err(error),
            }
        }
        Err(io::Error::new(
            io::ErrorKind::WouldBlock,
            format!("lock at {} is busy", path.display()),
        ))
    }
}

impl Drop for StaleLock {
    fn drop(&mut self) {
        // Only remove the file if it still holds this instance's own token.
        // A read failure (the file is already gone) needs no action; a
        // mismatched token means a waiter reclaimed this path as stale while
        // this instance was still legitimately alive, and the file now
        // belongs to that new holder — removing it here would release a
        // lock this instance never held.
        if fs::read_to_string(&self.path).is_ok_and(|content| content == self.token) {
            let _ = fs::remove_file(&self.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::time::Instant;

    #[test]
    fn a_free_path_is_acquired_immediately() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("held.lock");
        let lock = StaleLock::acquire(&path, Duration::from_secs(30)).unwrap();
        assert!(path.exists());
        drop(lock);
        assert!(!path.exists(), "drop must remove the lock file");
    }

    #[test]
    fn a_second_acquire_waits_for_the_first_to_drop() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("contended.lock");
        let first = StaleLock::acquire(&path, Duration::from_secs(30)).unwrap();
        let waiter_path = path.clone();
        let (ready_tx, ready_rx) = mpsc::channel();
        let handle = std::thread::spawn(move || {
            ready_tx.send(()).unwrap();
            let started = Instant::now();
            let second = StaleLock::acquire(&waiter_path, Duration::from_secs(30)).unwrap();
            (started.elapsed(), second)
        });
        ready_rx.recv().unwrap();
        std::thread::sleep(Duration::from_millis(100));
        drop(first);
        let (waited, _second) = handle.join().unwrap();
        assert!(
            waited >= Duration::from_millis(50),
            "second acquire returned before the first was dropped"
        );
    }

    #[test]
    fn a_lock_untouched_past_its_stale_age_is_stolen() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("abandoned.lock");
        fs::write(&path, b"").unwrap();
        let old = std::time::SystemTime::now() - Duration::from_secs(120);
        let file = fs::File::open(&path).unwrap();
        file.set_modified(old).unwrap();
        let lock = StaleLock::acquire(&path, Duration::from_millis(50));
        assert!(
            lock.is_ok(),
            "a lock past its staleness age must be reclaimed, not block forever"
        );
    }

    #[test]
    fn a_stolen_locks_original_holder_does_not_delete_the_new_holders_lock() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("stolen.lock");
        let original = StaleLock::acquire(&path, Duration::from_secs(30)).unwrap();
        // Simulate a waiter reclaiming this path as abandoned while
        // `original` is still alive: the file it created is gone, replaced
        // by a different holder's token.
        fs::remove_file(&path).unwrap();
        fs::write(&path, b"a-different-holders-token").unwrap();
        drop(original);
        assert!(
            path.exists(),
            "dropping a lock whose file no longer holds its own token must not \
             delete the lock a later holder is relying on"
        );
        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "a-different-holders-token"
        );
    }
}
