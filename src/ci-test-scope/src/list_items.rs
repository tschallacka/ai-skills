// MODE: DEV
// PACKAGE: PROD
//! Reads the canonical test/crate list from the REAL `run-tests.sh
//! --list-only` (shelled to, exactly as the bash original does via
//! `"$run_tests" --list-only`) -- never a second, driftable copy of "what
//! counts as a test." `run-tests.sh` is itself already wired onto its own
//! compiled binary (goal 15), so invoking the shell script transparently
//! benefits from that preference when a compiled `run-tests` binary exists;
//! this crate does not need its own separate binary-preference check here.

use std::path::Path;
use std::process::{Command, Stdio};

pub enum ListError {
    CommandFailed,
    Empty,
}

/// One repo-relative path per line, sorted, exactly as `run-tests.sh
/// --list-only` prints it -- order preserved, never re-sorted here.
///
/// `LC_ALL=C` is set explicitly on this subprocess: `run-tests.sh`'s own
/// shell-test discovery (`find ... | sort`, line ~162) is a BARE `sort`
/// that inherits whatever locale is ambient in its caller's environment,
/// rather than forcing C collation itself the way its crate-list `sort`
/// does (`LC_ALL=C sort`, line ~176, B203's own fix). The real bash
/// `ci-test-scope.sh` this crate ports already does `export LC_ALL=C` at
/// its own top BEFORE shelling to `run-tests.sh`, so that bare sort
/// inherits C collation there; reproducing the same explicit export here
/// is required for byte-identical, locale-independent ordering rather than
/// depending on whatever locale happens to be ambient when this compiled
/// binary itself is invoked.
pub fn list_items(repo_root: &Path) -> Result<Vec<String>, ListError> {
    let run_tests = repo_root.join("run-tests.sh");
    let output = Command::new("bash")
        .arg(&run_tests)
        .arg("--list-only")
        .current_dir(repo_root)
        .env("LC_ALL", "C")
        .stdin(Stdio::null())
        .output()
        .map_err(|_| ListError::CommandFailed)?;
    if !output.status.success() {
        return Err(ListError::CommandFailed);
    }
    let text = String::from_utf8_lossy(&output.stdout);
    let items: Vec<String> = text
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.to_string())
        .collect();
    if items.is_empty() {
        return Err(ListError::Empty);
    }
    Ok(items)
}
