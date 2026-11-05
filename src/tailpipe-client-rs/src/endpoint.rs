// MODE: DEV
// PACKAGE: PROD
//! The `$TAILPIPE_HOME`-rooted default endpoint path, and its fallback for a
//! `$HOME` too long to fit a unix socket address. Lives here (not in
//! tailpipe-mcp, which linked this as a library) so a shell-side default-path
//! helper, should one ever be added to this crate's own CLI, shares the
//! identical fallback rather than risking a second, silently divergent copy
//! (B5).

use std::path::{Path, PathBuf};

/// Unix socket paths have a small platform-defined limit (`sun_path` is 104
/// bytes on macOS, 108 on Linux, the null terminator included) -- a real
/// bind/connect failure ("path must be shorter than SUN_LEN"), not a
/// hypothetical: measured live against a deeply nested $HOME (a disposable
/// per-run HOME under a benchmark harness) at 126 bytes, 18 over the Linux
/// limit (B5). Conservative bound, leaving headroom below the real platform
/// limits rather than targeting them exactly.
const SUN_PATH_SAFE_LIMIT: usize = 90;

fn fits(path: &Path) -> bool {
    !cfg!(unix) || path.to_string_lossy().len() <= SUN_PATH_SAFE_LIMIT
}

/// Distinguishes one preferred root from another in the shared `/tmp`
/// fallback namespace, the same job chat-client-rs's own `state_tag` does for
/// its control socket: a collision costs a wrong-server connection attempt
/// (refused by the protocol, not a security property), not silent data
/// sharing between unrelated homes.
fn root_tag(preferred: &Path) -> String {
    // FNV-1a, matching chat-client-rs's own short-path fallback tag.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in preferred.as_os_str().as_encoded_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    format!("{:08x}", (hash >> 32) as u32)
}

/// Where the endpoint goes when the preferred, $HOME-rooted path would not
/// fit a unix socket address. `/tmp` rather than `std::env::temp_dir()`,
/// because a long `$TMPDIR` is the same kind of condition that got us here;
/// keyed by the preferred path's own tag so two distinct homes still resolve
/// to two distinct fallback endpoints instead of colliding on one.
fn short_endpoint(preferred: &Path) -> PathBuf {
    PathBuf::from("/tmp")
        .join(format!("tsch-ai-skills-tailpipe-{}", root_tag(preferred)))
        .join("tailpipe.sock")
}

/// The pure decision, taking the `$TAILPIPE_HOME` candidate explicitly
/// rather than reading it from the environment -- lets a test compute the
/// exact same answer for a home it chose itself, without mutating this
/// process's own env (tests run as parallel threads in one process, where an
/// env write lands under a sibling; the same reason planning-server's own
/// `endpoint::resolve` takes its runtime dir as a parameter).
pub fn resolve_endpoint_from(home: &Path) -> PathBuf {
    let preferred = home.join("tailpipe.sock");
    if fits(&preferred) {
        preferred
    } else {
        short_endpoint(&preferred)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_short_home_keeps_the_preferred_path() {
        let home = Path::new("/home/alice/.config/tsch-ai-skills/tailpipe");
        let resolved = resolve_endpoint_from(home);
        assert_eq!(resolved, home.join("tailpipe.sock"));
    }

    #[test]
    fn a_long_home_falls_back_to_a_short_tagged_tmp_path() {
        // The exact path this bug was filed from: a disposable per-cell HOME
        // under the skill-adoption benchmark harness.
        let home = Path::new(
            "/home/tschallacka/.cache/ai-skills-benchmark/skill-adoption/cell.XXXXXXXXXX/home/.config/tsch-ai-skills/tailpipe",
        );
        assert!(
            !fits(&home.join("tailpipe.sock")),
            "fixture must actually exceed the safe limit, or this test proves nothing"
        );
        let resolved = resolve_endpoint_from(home);
        assert!(
            fits(&resolved),
            "fallback must itself always fit: {}",
            resolved.display()
        );
        assert!(resolved.starts_with("/tmp"));
        assert_eq!(resolved.file_name().unwrap(), "tailpipe.sock");
    }

    #[test]
    fn two_distinct_long_homes_fall_back_to_two_distinct_endpoints() {
        let a = Path::new("/home/alice/very/deeply/nested/workspace/root/for/testing/.config/tsch-ai-skills/tailpipe");
        let b = Path::new("/home/bob/very/deeply/nested/workspace/root/for/testing/.config/tsch-ai-skills/tailpipe");
        assert_ne!(resolve_endpoint_from(a), resolve_endpoint_from(b));
    }

    #[test]
    fn the_same_long_home_resolves_to_the_same_endpoint_every_time() {
        let home = Path::new(
            "/home/tschallacka/.cache/ai-skills-benchmark/skill-adoption/cell.XXXXXXXXXX/home/.config/tsch-ai-skills/tailpipe",
        );
        assert_eq!(resolve_endpoint_from(home), resolve_endpoint_from(home));
    }
}
