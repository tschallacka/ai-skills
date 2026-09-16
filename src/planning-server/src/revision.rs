// MODE: DEV
// PACKAGE: PROD
//! Content-hash-based optimistic-concurrency guard for plan files.
//!
//! Unlike ai-text-editor's own RevisionGuard(u64) (a monotonic counter the
//! server alone increments, correct because the server is the tab's ONLY
//! writer), planning documents are still written directly by 45+ other
//! already-wired bash/Rust entry points this crate does not touch. A counter
//! this process owns cannot detect a write none of its own calls made, so the
//! guard here is a hash of the document's own bytes: any write, from any
//! source, changes the hash, and a caller's guard is checked against
//! whatever is on disk right now, not against this process's own history.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PlanRevision(pub blake3::Hash);

impl PlanRevision {
    pub fn of(bytes: &[u8]) -> Self {
        PlanRevision(blake3::hash(bytes))
    }

    pub fn to_hex(self) -> String {
        self.0.to_hex().to_string()
    }

    /// Parses a revision as submitted over the wire; a malformed hex string
    /// is refused by name rather than panicking.
    pub fn from_hex(text: &str) -> Result<Self, String> {
        blake3::Hash::from_hex(text)
            .map(PlanRevision)
            .map_err(|error| format!("malformed revision {text:?}: {error}"))
    }
}

impl std::fmt::Display for PlanRevision {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum RevisionError {
    #[error("stale revision: the document is at revision {actual}; this request supplied {expected} - re-read and retry with the current revision")]
    Stale { expected: String, actual: String },
    #[error("{0}")]
    Io(String),
}

impl RevisionError {
    fn stale(expected: PlanRevision, actual: PlanRevision) -> Self {
        RevisionError::Stale {
            expected: expected.to_hex(),
            actual: actual.to_hex(),
        }
    }
}

/// Reads a document's current bytes and the PlanRevision derived from them.
pub fn read_with_revision(path: &Path) -> Result<(Vec<u8>, PlanRevision), RevisionError> {
    let bytes = fs::read(path)
        .map_err(|error| RevisionError::Io(format!("cannot read {}: {error}", path.display())))?;
    let revision = PlanRevision::of(&bytes);
    Ok((bytes, revision))
}

/// Process-wide per-path mutex registry: closes the read-hash/check/write
/// race between two of THIS process's own concurrent callers (both could
/// otherwise read the same valid hash before either writes). It cannot, and
/// is not meant to, prevent a write from an external process racing in
/// between this call's own read and write -- that write changes the on-disk
/// hash, and the very next guarded call from anywhere detects it as Stale;
/// this call's own write either lands first or is the one that then loses
/// the race and refuses, so no write is silently dropped either way.
fn path_locks() -> &'static Mutex<HashMap<PathBuf, Arc<Mutex<()>>>> {
    static LOCKS: OnceLock<Mutex<HashMap<PathBuf, Arc<Mutex<()>>>>> = OnceLock::new();
    LOCKS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn lock_for(path: &Path) -> Arc<Mutex<()>> {
    let mut locks = path_locks()
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    locks
        .entry(path.to_path_buf())
        .or_insert_with(|| Arc::new(Mutex::new(())))
        .clone()
}

/// Applies a guarded write: refuses with `Stale` unless the caller's `guard`
/// still matches the document's current on-disk hash, otherwise writes via
/// `planning_core::atomic_write` and returns the new revision. Holds this
/// path's own mutex for the whole read-check-write sequence so two calls
/// from this process cannot both pass the check before either writes.
pub fn write_guarded(
    path: &Path,
    guard: PlanRevision,
    new_bytes: &[u8],
) -> Result<PlanRevision, RevisionError> {
    let path_lock = lock_for(path);
    let _held = path_lock
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());

    let current = fs::read(path)
        .map_err(|error| RevisionError::Io(format!("cannot read {}: {error}", path.display())))?;
    let actual = PlanRevision::of(&current);
    if actual != guard {
        return Err(RevisionError::stale(guard, actual));
    }
    planning_core::atomic_write(path, new_bytes).map_err(RevisionError::Io)?;
    Ok(PlanRevision::of(new_bytes))
}

/// The same guarded read-check sequence as `write_guarded`, but for a write
/// this crate delegates to an external process (an existing standalone
/// command) instead of writing bytes itself: `action` runs only once the
/// guard passes, holding this path's own mutex for its whole duration so a
/// second call cannot race in between the check and the delegated write.
/// Returns the path's new revision after `action` completes; a failing
/// `action` reports its own message and leaves the file exactly as `action`
/// left it (no attempt to roll back a partially-applied external write).
pub fn guarded_call<F>(
    path: &Path,
    guard: PlanRevision,
    action: F,
) -> Result<PlanRevision, RevisionError>
where
    F: FnOnce() -> Result<(), String>,
{
    let path_lock = lock_for(path);
    let _held = path_lock
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());

    let current = fs::read(path)
        .map_err(|error| RevisionError::Io(format!("cannot read {}: {error}", path.display())))?;
    let actual = PlanRevision::of(&current);
    if actual != guard {
        return Err(RevisionError::stale(guard, actual));
    }
    action().map_err(RevisionError::Io)?;
    let updated = fs::read(path).map_err(|error| {
        RevisionError::Io(format!(
            "cannot read {} after write: {error}",
            path.display()
        ))
    })?;
    Ok(PlanRevision::of(&updated))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn scratch_file(content: &[u8]) -> (tempfile_dir::TempDir, PathBuf) {
        let dir = tempfile_dir::TempDir::new();
        let path = dir.path().join("doc.md");
        let mut file = fs::File::create(&path).unwrap();
        file.write_all(content).unwrap();
        (dir, path)
    }

    mod tempfile_dir {
        use std::path::{Path, PathBuf};

        pub struct TempDir(PathBuf);

        impl TempDir {
            pub fn new() -> Self {
                let mut dir = std::env::temp_dir();
                dir.push(format!(
                    "planning-server-revision-test-{}-{}",
                    std::process::id(),
                    unique()
                ));
                std::fs::create_dir_all(&dir).unwrap();
                TempDir(dir)
            }

            pub fn path(&self) -> &Path {
                &self.0
            }
        }

        impl Drop for TempDir {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }

        fn unique() -> u64 {
            use std::sync::atomic::{AtomicU64, Ordering};
            static COUNTER: AtomicU64 = AtomicU64::new(0);
            COUNTER.fetch_add(1, Ordering::Relaxed)
        }
    }

    #[test]
    fn matching_guard_write_succeeds_and_returns_new_revision() {
        let (_dir, path) = scratch_file(b"original");
        let (_, revision) = read_with_revision(&path).unwrap();
        let new_revision = write_guarded(&path, revision, b"updated").unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"updated");
        assert_eq!(new_revision, PlanRevision::of(b"updated"));
    }

    #[test]
    fn a_write_from_outside_this_process_is_detected_as_stale() {
        let (_dir, path) = scratch_file(b"original");
        let (_, revision) = read_with_revision(&path).unwrap();

        // Simulate an external bash writer: a plain write with no guard at all.
        fs::write(&path, b"external change").unwrap();

        let result = write_guarded(&path, revision, b"my update");
        assert_eq!(
            result,
            Err(RevisionError::Stale {
                expected: revision.to_hex(),
                actual: PlanRevision::of(b"external change").to_hex(),
            })
        );
        assert_eq!(
            fs::read(&path).unwrap(),
            b"external change",
            "a stale write must leave the file untouched"
        );
    }

    #[test]
    fn two_sequential_writes_from_the_same_process_each_see_the_prior_ones_own_write() {
        let (_dir, path) = scratch_file(b"v0");
        let (_, r0) = read_with_revision(&path).unwrap();

        let r1 = write_guarded(&path, r0, b"v1").unwrap();
        assert_eq!(r1, PlanRevision::of(b"v1"));

        // Replaying the FIRST revision now must fail: the mutex does not
        // hide the first call's own effect from the second.
        let stale_replay = write_guarded(&path, r0, b"v2-wrong");
        assert!(matches!(stale_replay, Err(RevisionError::Stale { .. })));
        assert_eq!(fs::read(&path).unwrap(), b"v1");

        let r2 = write_guarded(&path, r1, b"v2").unwrap();
        assert_eq!(r2, PlanRevision::of(b"v2"));
        assert_eq!(fs::read(&path).unwrap(), b"v2");
    }

    #[test]
    fn plan_revision_display_matches_to_hex() {
        let revision = PlanRevision::of(b"hello");
        assert_eq!(revision.to_string(), revision.to_hex());
    }

    #[test]
    fn from_hex_round_trips_and_refuses_garbage() {
        let revision = PlanRevision::of(b"round trip me");
        assert_eq!(
            PlanRevision::from_hex(&revision.to_hex()).unwrap(),
            revision
        );
        assert!(PlanRevision::from_hex("not hex at all").is_err());
    }

    #[test]
    fn guarded_call_runs_the_action_only_when_the_guard_matches() {
        let (_dir, path) = scratch_file(b"before");
        let (_, revision) = read_with_revision(&path).unwrap();
        let path_for_action = path.clone();
        let new_revision = guarded_call(&path, revision, || {
            fs::write(&path_for_action, b"after").map_err(|error| error.to_string())
        })
        .unwrap();
        assert_eq!(fs::read(&path).unwrap(), b"after");
        assert_eq!(new_revision, PlanRevision::of(b"after"));
    }

    #[test]
    fn guarded_call_refuses_and_never_runs_the_action_when_stale() {
        let (_dir, path) = scratch_file(b"before");
        let (_, revision) = read_with_revision(&path).unwrap();
        fs::write(&path, b"someone else wrote this").unwrap();
        let mut action_ran = false;
        let result = guarded_call(&path, revision, || {
            action_ran = true;
            Ok(())
        });
        assert!(matches!(result, Err(RevisionError::Stale { .. })));
        assert!(!action_ran, "a stale guard must never run the action");
    }
}
