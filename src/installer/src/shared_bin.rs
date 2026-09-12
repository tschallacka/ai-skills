// MODE: DEV
// PACKAGE: PROD
//! T72: every skill that ships a compiled binary used to get its own copy
//! under `<skill>/bin/<target-triple>/`, installed once per skill per agent
//! root -- three agent roots each carrying their own `rjq`-sized binary. One
//! shared location, matched by `planning/scripts/lib/crypt/plan_bin_dir.sh`'s
//! own already-shipped fallback (`${XDG_CONFIG_HOME:-$HOME/.config}/tsch-ai-skills/bin`),
//! replaces every one of those per-skill copies -- retiring the whole
//! per-skill `bin/` convention, not just rjq's. No back-compat: an existing
//! per-skill-bin install is fixed by re-running the installer (Tschallacka,
//! 2026-09-10).
//!
//! Flat, not `bin/<triple>/`: the shared directory only ever holds THIS
//! host's own binaries (a install never writes another platform's), so a
//! triple subdirectory there would be pure noise -- the same reasoning
//! `plan_bin_dir` already applies to it.

use std::path::{Path, PathBuf};

pub fn shared_bin_dir(home: &Path) -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"));
    base.join("tsch-ai-skills").join("bin")
}

/// `collect_relative_files` (install.rs) only ever emits a `bin/...` entry
/// as `bin/<this-host's-own-triple>/<file>`, having already dropped every
/// other platform's subdirectory -- so any relative path starting with
/// `bin/` unambiguously names a shared binary, and the filename is
/// everything after the triple segment.
pub fn shared_binary_filename(relative: &str) -> Option<&str> {
    let rest = relative.strip_prefix("bin/")?;
    let (_triple, filename) = rest.split_once('/')?;
    (!filename.is_empty()).then_some(filename)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_bin_entry_names_its_filename_past_the_triple() {
        assert_eq!(
            shared_binary_filename("bin/x86_64-unknown-linux-musl/rjq"),
            Some("rjq")
        );
        assert_eq!(
            shared_binary_filename("bin/aarch64-apple-darwin/chat-mcp"),
            Some("chat-mcp")
        );
    }

    #[test]
    fn a_non_bin_entry_names_nothing() {
        assert_eq!(shared_binary_filename("SKILL.md"), None);
        assert_eq!(shared_binary_filename("scripts/run.sh"), None);
        assert_eq!(shared_binary_filename("bin/"), None);
        assert_eq!(shared_binary_filename("bin/onlyatriple"), None);
    }

    // No test here mutates XDG_CONFIG_HOME: it is a process-global env var,
    // and only one module in this crate (plan_migration, under its own
    // ENV_LOCK) takes on that cross-test race today. shared_bin_dir's
    // fallback-when-unset shape is identical to plan_migration::default_root's
    // already-covered one; adding a second, unlocked mutator here would race
    // it rather than add real coverage.
}
