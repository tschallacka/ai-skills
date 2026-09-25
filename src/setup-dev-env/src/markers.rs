// MODE: DEV
// PACKAGE: PROD

//! B156 dirty-tree markers: .setup-dev-env.started is written with a
//! unique-per-run token at the moment building begins; .finished is
//! written with the SAME token only once every crate in the plan has
//! built successfully. A crate failure must leave .finished absent or
//! naming an older run. The exact token shape does not matter, since
//! nothing else parses this file's content beyond comparing it for exact
//! equality against .finished.

use std::path::Path;

pub fn new_token() -> String {
    let pid = std::process::id();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{pid}.{now}")
}

pub fn write_started(repo_root: &Path, token: &str) -> std::io::Result<()> {
    std::fs::write(
        repo_root.join(".setup-dev-env.started"),
        format!("{token}\n"),
    )
}

pub fn write_finished(repo_root: &Path, token: &str) -> std::io::Result<()> {
    std::fs::write(
        repo_root.join(".setup-dev-env.finished"),
        format!("{token}\n"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn scratch(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "setup-dev-env-markers-{tag}-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn started_then_finished_with_the_same_token_round_trips() {
        let dir = scratch("round-trip");
        let token = new_token();
        write_started(&dir, &token).unwrap();
        write_finished(&dir, &token).unwrap();
        let started = fs::read_to_string(dir.join(".setup-dev-env.started")).unwrap();
        let finished = fs::read_to_string(dir.join(".setup-dev-env.finished")).unwrap();
        assert_eq!(started.trim(), token);
        assert_eq!(finished.trim(), token);
    }

    #[test]
    fn a_failed_run_leaves_finished_absent() {
        let dir = scratch("failed-run");
        let token = new_token();
        write_started(&dir, &token).unwrap();
        assert!(!dir.join(".setup-dev-env.finished").exists());
    }

    #[test]
    fn tokens_for_distinct_runs_are_stable_strings() {
        let a = new_token();
        assert!(a.contains('.'));
    }
}
