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
    let base = xdg_config_home()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"));
    base.join("tsch-ai-skills").join("bin")
}

// B327: `shared_bin_dir` takes `home` as an explicit parameter specifically so
// a caller (a test, above all) can pass an isolated tempdir -- but reading
// the real XDG_CONFIG_HOME first silently defeated that whenever the ambient
// process environment happened to have it set, or whenever some OTHER test
// in the same process (plan_migration's own tests do this) mutated it: every
// test calling `shared_bin_dir(my_own_tempdir)` shared ONE real directory
// instead, and raced any other such test running at the same moment.
//
// The fix is to make a test build never read the real environment at all,
// by default: `resolve(key)` answers `None` (unset, so `home` wins) unless a
// test explicitly opts in with `set_override`, which is only what the one
// or two tests that specifically verify the env-override behavior itself
// need to do. Every other test -- which is most of them -- needs no change
// at all, rather than needing to remember to force each var unset itself.
// `cargo test` gives each `#[test]` its own OS thread (joined and discarded
// when that one test ends, never reused for a different test), so this
// needs no lock and no restore-on-drop bookkeeping: an override cannot
// outlive or reach any other test. Production code is untouched either way
// -- this module's own `#[cfg(test)]` gate means the override path does not
// exist in a non-test build, which keeps reading the real env var exactly
// as it always did.
#[cfg(test)]
pub(crate) mod test_env {
    use std::cell::RefCell;
    use std::collections::HashMap;

    thread_local! {
        static OVERRIDES: RefCell<HashMap<&'static str, Option<String>>> = RefCell::new(HashMap::new());
    }

    /// Opt THIS thread's `resolve(key)` into simulating a specific real
    /// environment: `Some(v)` simulates `key=v` being set, `None` simulates
    /// it being unset -- for the one or two tests that specifically verify
    /// `shared_bin_dir`'s/`default_root`'s env-override behavior itself.
    /// Every other test needs no call here at all: `resolve` already
    /// defaults to "unset" without one.
    pub(crate) fn set_override(key: &'static str, value: Option<&str>) {
        OVERRIDES.with(|cell| {
            cell.borrow_mut().insert(key, value.map(str::to_string));
        });
    }

    /// What a caller should treat `key` as being set to: this thread's own
    /// override if one was set via `set_override`, else `None` (unset) --
    /// deliberately never the real process environment, so a test that
    /// never calls `set_override` is automatically isolated from it.
    pub(crate) fn resolve(key: &'static str) -> Option<String> {
        OVERRIDES.with(|cell| cell.borrow().get(key).cloned().flatten())
    }
}

#[cfg(test)]
fn xdg_config_home() -> Option<String> {
    test_env::resolve("XDG_CONFIG_HOME")
}

#[cfg(not(test))]
fn xdg_config_home() -> Option<String> {
    std::env::var("XDG_CONFIG_HOME").ok()
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

    // B327: no test here needs to touch `test_env::set_override` -- these two
    // tests exercise `shared_binary_filename`, a pure string function with no
    // env involvement at all. Every test elsewhere in this crate that calls
    // `shared_bin_dir` with an isolated tempdir is already isolated from the
    // real XDG_CONFIG_HOME by default -- `resolve` never falls through to the
    // real environment in a test build, so only the one or two tests that
    // specifically verify the override behavior itself need to call
    // `set_override` at all.
}
