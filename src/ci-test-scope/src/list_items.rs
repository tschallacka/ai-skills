// MODE: DEV
// PACKAGE: PROD
//! Reads the canonical test/crate list by shelling to the real
//! `run-tests.sh --list-only` -- never a second, driftable copy of "what
//! counts as a test."

use std::path::Path;
use std::process::{Command, Stdio};

pub enum ListError {
    CommandFailed,
    Empty,
}

/// One repo-relative path per line, sorted, exactly as `run-tests.sh
/// --list-only` prints it -- order preserved, never re-sorted here.
///
/// `LC_ALL=C` is set explicitly on this subprocess: its shell-test discovery
/// step is a bare `sort` that would otherwise inherit whatever locale is
/// ambient in the caller's environment, rather than forcing C collation
/// itself. Setting it here keeps ordering byte-identical and
/// locale-independent regardless of the ambient locale.
pub fn list_items(repo_root: &Path) -> Result<Vec<String>, ListError> {
    let run_tests = repo_root.join("run-tests.sh");
    let output = Command::new(crate::shell::bash())
        .arg(crate::shell::script_arg(&run_tests))
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
