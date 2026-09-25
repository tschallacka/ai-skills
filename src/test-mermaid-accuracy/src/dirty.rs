// MODE: DEV
// PACKAGE: PROD

//! Reproduces lib-test.sh's own `_t_dev_env_dirty_reason` precondition
//! independently (the compiled binary never sources lib-test.sh): a build
//! started via .setup-dev-env.started with no matching .setup-dev-env.finished
//! token means the tree is in an unknown, possibly partial state.
//!
//! Takes an explicit repo-root parameter (mirroring
//! src/setup-dev-env/src/markers.rs's own write_started/write_finished
//! signatures) so a test can plant a scratch pair without ever touching this
//! live repository's own real marker files.

use std::fs;
use std::path::Path;

pub fn dirty_reason(repo_root: &Path) -> Option<String> {
    let started = repo_root.join(".setup-dev-env.started");
    let finished = repo_root.join(".setup-dev-env.finished");
    if !started.is_file() {
        return None;
    }
    let started_token = fs::read_to_string(&started).unwrap_or_default();
    let started_token = started_token.trim();
    let finished_token = fs::read_to_string(&finished).unwrap_or_default();
    let finished_token = finished_token.trim();
    if !started_token.is_empty() && started_token == finished_token {
        return None;
    }
    Some(
        "a setup-dev-env.sh run started and never finished (or finished a different run) -- \
the build tree is in an unknown, possibly partial state. Finish it, then re-run: \
./setup-dev-env.sh"
            .to_string(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "test-mermaid-accuracy-dirty-{tag}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn no_started_file_is_clean() {
        let dir = scratch("no-started");
        assert!(dirty_reason(&dir).is_none());
    }

    #[test]
    fn matching_tokens_are_clean() {
        let dir = scratch("matching");
        fs::write(dir.join(".setup-dev-env.started"), "abc\n").unwrap();
        fs::write(dir.join(".setup-dev-env.finished"), "abc\n").unwrap();
        assert!(dirty_reason(&dir).is_none());
    }

    #[test]
    fn started_with_no_finished_is_dirty() {
        let dir = scratch("no-finished");
        fs::write(dir.join(".setup-dev-env.started"), "abc\n").unwrap();
        let reason = dirty_reason(&dir).unwrap();
        assert!(reason.contains("started and never finished"));
    }

    #[test]
    fn mismatched_tokens_are_dirty() {
        let dir = scratch("mismatched");
        fs::write(dir.join(".setup-dev-env.started"), "abc\n").unwrap();
        fs::write(dir.join(".setup-dev-env.finished"), "xyz\n").unwrap();
        assert!(dirty_reason(&dir).is_some());
    }
}
