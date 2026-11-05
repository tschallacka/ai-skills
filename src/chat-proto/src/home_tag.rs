// MODE: DEV
// PACKAGE: PROD
//! A short, stable identifier for one chat state directory (`AI_CHAT_HOME`),
//! so a client doing beacon discovery can tell an announcing server apart
//! from an unrelated one on the same host/LAN without putting the real
//! filesystem path on the wire.
//!
//! The beacon is a UDP broadcast with no concept of `AI_CHAT_HOME` at all: it
//! only ever carried a connect address. A client with no saved session and no
//! cached server yet (a brand-new identity, under a deliberately distinct
//! `AI_CHAT_HOME`) had no way to prefer "the server for MY state directory"
//! over any other server announcing on the same beacon port -- it simply
//! joined whichever one answered first. Tagging the beacon with this value,
//! and comparing it against the caller's own, is what lets `resolve_server`
//! (chat-client-rs) prefer a same-origin match before falling back to
//! whatever else is there.

use std::path::Path;

/// FNV-1a 64-bit of the canonicalized path, folded to 12 hex characters.
/// Not a cryptographic hash and not meant to be one: it is a discovery hint,
/// never an authorization check (TOFU certificate pinning is what actually
/// keeps a connection honest), so collisions or a determined adversary
/// recovering the path are both outside what this needs to resist. Two
/// processes given the same `AI_CHAT_HOME` value compute the same tag without
/// either ever having to send the other its real path.
///
/// Falls back to hashing the path as given when it cannot be canonicalized
/// (most commonly: the directory does not exist yet) -- still stable for two
/// processes started with the identical string, just not resilient to a
/// symlink or relative/absolute spelling difference in that case.
pub fn home_tag(path: &Path) -> String {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let bytes = canonical.to_string_lossy();
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{:012x}", hash & 0x0000_ffff_ffff_ffff)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Both tests below built their scratch dir from only std::process::id(),
    /// which is identical across every thread of this one test binary --
    /// cargo test runs them concurrently, so the two names collided and one
    /// test's cleanup `remove_dir_all` could delete the shared parent while
    /// the other was still creating a subdirectory inside it (observed as
    /// `create_dir_all(&b).unwrap()` panicking with NotFound on a Windows
    /// CI leg). A per-test counter, alongside the pid, makes every scratch
    /// dir distinct regardless of thread interleaving.
    fn unique() -> u64 {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    #[test]
    fn the_same_path_always_produces_the_same_tag() {
        let dir =
            std::env::temp_dir().join(format!("home-tag-test-{}-{}", std::process::id(), unique()));
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(home_tag(&dir), home_tag(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn different_paths_produce_different_tags() {
        let base =
            std::env::temp_dir().join(format!("home-tag-test-{}-{}", std::process::id(), unique()));
        let a = base.join("a");
        let b = base.join("b");
        std::fs::create_dir_all(&a).unwrap();
        std::fs::create_dir_all(&b).unwrap();
        assert_ne!(home_tag(&a), home_tag(&b));
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn a_nonexistent_path_still_produces_a_stable_tag() {
        let path = Path::new("/this/path/does/not/exist/on/any/machine");
        assert_eq!(home_tag(path), home_tag(path));
    }

    #[test]
    fn the_tag_is_twelve_lowercase_hex_characters() {
        let tag = home_tag(Path::new("/tmp"));
        assert_eq!(tag.len(), 12);
        assert!(tag
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }

    #[test]
    fn a_trailing_slash_resolves_to_the_same_tag_as_without_it() {
        let dir = std::env::temp_dir().join(format!("home-tag-test-slash-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let with_slash = format!("{}/", dir.display());
        assert_eq!(home_tag(&dir), home_tag(Path::new(&with_slash)));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
