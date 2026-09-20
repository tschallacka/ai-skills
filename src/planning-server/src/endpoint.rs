// MODE: DEV
// PACKAGE: PROD
//! Where the planning-server daemon's socket lives.
//!
//! One shared daemon serves every plan directory (unlike ai-text-editor,
//! which starts one server per open file/tab) -- each Request carries its own
//! `plan_dir`, so the socket path itself needs no per-plan identity, only a
//! single well-known location this session's client and server both resolve
//! the same way.

use std::path::{Path, PathBuf};

// Unix domain socket paths have a small platform-defined limit (sun_path is
// 104 bytes on macOS, 108 on Linux, the null terminator included) --
// ai-text-editor/src/transport.rs already hit this and falls back to a
// short root under plain /tmp rather than honor a long XDG_RUNTIME_DIR (or
// TMPDIR, which -- once TMPDIR is itself the long path -- std::temp_dir()
// would only reintroduce). Found here the same way: a real bind failure
// ("path must be shorter than SUN_LEN") under this repo's own nix-shell
// scratch layout, not a hypothetical. Conservative bound: 90, leaving room
// for the socket's own filename under the computed root.
const SUN_PATH_SAFE_LIMIT: usize = 90;

fn preferred_root(runtime_dir: &Path) -> PathBuf {
    runtime_dir.join("tsch-ai-skills-planning-server")
}

/// Mirrors ai-text-editor/src/transport.rs's own short_root: falls back to
/// plain /tmp (never TMPDIR, which being long is the very condition that
/// got us here) but still hashes the ORIGINAL runtime_dir into the
/// directory name. Without that hash every caller whose preferred root was
/// too long would collapse onto the identical fallback path -- harmless for
/// the single real daemon a real host runs, but wrong the moment more than
/// one caller (this crate's own concurrent integration tests, most
/// concretely) legitimately wants its OWN distinct socket and each computes
/// its own runtime_dir to get one.
fn short_root(runtime_dir: &Path) -> PathBuf {
    let root_key = blake3::hash(runtime_dir.to_string_lossy().as_bytes()).to_hex();
    #[cfg(unix)]
    let owner = unsafe { libc::getuid() }.to_string();
    #[cfg(not(unix))]
    let owner = std::env::var("USERNAME").unwrap_or_else(|_| "user".into());
    #[cfg(unix)]
    let base = PathBuf::from("/tmp");
    #[cfg(not(unix))]
    let base = std::env::temp_dir();
    base.join(format!(
        "tsch-ai-skills-planning-server-{owner}-{}",
        &root_key[..8]
    ))
}

/// Only a unix domain socket has a length limit. Elsewhere the endpoint is a
/// plain discovery file (see `transport`), which any ordinary path can hold,
/// and a Windows temp directory plus this file's name already exceeds 90.
fn fits(root: &Path) -> bool {
    !cfg!(unix) || root.join("planning-server.sock").to_string_lossy().len() <= SUN_PATH_SAFE_LIMIT
}

/// The pure decision this module makes, taking the candidate runtime
/// directory explicitly rather than reading it from the environment --
/// lets a test compute the exact same answer for a runtime dir it set as a
/// CHILD process's own environment variable, without mutating its own
/// process-wide env (unsafe on this toolchain, and process-global besides).
pub fn resolve(runtime_dir: &Path) -> PathBuf {
    let preferred = preferred_root(runtime_dir);
    let root = if fits(&preferred) {
        preferred
    } else {
        short_root(runtime_dir)
    };
    root.join("planning-server.sock")
}

pub fn socket_path() -> PathBuf {
    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    resolve(&runtime_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn short_root_is_always_well_under_the_safe_limit() {
        let runtime_dir = Path::new("/tmp/some/very/long/runtime/dir/that/would/not/fit");
        let root = short_root(runtime_dir);
        assert!(
            fits(&root),
            "the fallback root must itself always fit: {}",
            root.display()
        );
        // Where the limit applies (a unix socket), that root is the one kept
        // deliberately short by living directly under /tmp.
        #[cfg(unix)]
        assert!(root.starts_with("/tmp"));
    }

    #[test]
    fn a_root_over_the_limit_falls_back_only_where_the_limit_applies() {
        let long = Path::new(
            "/tmp/one/long/enough/runtime/dir/to/trigger/fallback/aaaaaaaaaaaaaaaaaaaaaa",
        );
        let resolved = resolve(long);
        if cfg!(unix) {
            assert!(
                resolved.starts_with(short_root(long)),
                "an over-long unix socket path must move to the short root: {}",
                resolved.display()
            );
        } else {
            assert!(
                resolved.starts_with(preferred_root(long)),
                "a discovery file has no length limit, so the preferred root stays: {}",
                resolved.display()
            );
        }
    }

    #[test]
    fn two_different_runtime_dirs_never_collapse_onto_the_same_fallback_root() {
        // Regression: a fallback keyed only on uid, not on the runtime dir,
        // made every caller whose preferred root was too long collide on one
        // shared socket -- harmless for a single real daemon, but a genuine
        // cross-test collision for this crate's own concurrent integration
        // tests, found via a real run-tests.sh failure, not a unit test.
        let a = short_root(Path::new(
            "/tmp/one/long/enough/runtime/dir/to/trigger/fallback/a",
        ));
        let b = short_root(Path::new(
            "/tmp/one/long/enough/runtime/dir/to/trigger/fallback/b",
        ));
        assert_ne!(
            a, b,
            "distinct runtime dirs must fall back to distinct roots"
        );
    }
}
