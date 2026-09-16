// MODE: DEV
// PACKAGE: PROD

//! refuse_if_dev_env_dirty and bootstrap_generated: the two pre-flight
//! checks run-tests.sh performs before any test runs, both shelling out to
//! the same real scripts the bash original does rather than reimplementing
//! their own logic.

use std::env;
use std::path::Path;
use std::process::Command;

fn which(program: &str) -> bool {
    let Some(path_var) = env::var_os("PATH") else {
        return false;
    };
    env::split_paths(&path_var).any(|dir| dir.join(program).is_file())
}

/// Returns Err(message) with the exact multi-line refusal when a prior
/// setup-dev-env.sh run started and never finished (or finished a different
/// run), leaving the build tree in an unknown, possibly partial state.
pub fn refuse_if_dev_env_dirty(repo_root: &Path) -> Result<(), String> {
    let started_path = repo_root.join(".setup-dev-env.started");
    let Ok(started_token) = std::fs::read_to_string(&started_path) else {
        return Ok(());
    };
    let started_token = started_token.trim();
    let finished_path = repo_root.join(".setup-dev-env.finished");
    let finished_token = std::fs::read_to_string(&finished_path).unwrap_or_default();
    let finished_token = finished_token.trim();
    if !started_token.is_empty() && started_token == finished_token {
        return Ok(());
    }
    Err(concat!(
        "run-tests.sh: a setup-dev-env.sh run started and never finished (or finished a\n",
        "  different run) -- the build tree is in an unknown, possibly partial\n",
        "  state, which is the leading suspect behind this suite failing then\n",
        "  passing on an identical tree (B156). Finish it, then re-run:\n",
        "    ./setup-dev-env.sh\n",
    )
    .to_string())
}

/// Builds the five bundled plan-*-lib.sh files if any are missing,
/// generates planning/REVIEWER.md if missing, unconditionally regenerates
/// PORTABILITY.md, and resolves rjq -- exiting with the exact message (Err)
/// only when rjq is still missing afterward. Returns the directory to
/// prepend to child processes' own PATH when the fallback found one
/// (`Ok(None)` when rjq was already on PATH and nothing needs prepending);
/// mutating this process's own environment is deliberately avoided --
/// std::env::set_var is unsafe on this toolchain, and global mutation is
/// unsound anyway once the signal-handling thread is running.
pub fn bootstrap_generated(repo_root: &Path) -> Result<Option<String>, String> {
    const LIBS: [&str; 5] = [
        "plan-core-lib.sh",
        "plan-crypt-lib.sh",
        "plan-document-lib.sh",
        "plan-progress-lib.sh",
        "plan-table-lib.sh",
    ];
    let missing = LIBS
        .iter()
        .any(|lib| !repo_root.join("planning/scripts").join(lib).is_file());
    if missing {
        let _ = Command::new(repo_root.join("planning/scripts/build-plan-libs.sh"))
            .current_dir(repo_root)
            .status();
    }
    if !repo_root.join("planning/REVIEWER.md").is_file() {
        let _ = Command::new(repo_root.join("planning/scripts/generate-reviewer.sh"))
            .current_dir(repo_root)
            .status();
    }
    let _ = Command::new(repo_root.join("generate-portability.sh"))
        .current_dir(repo_root)
        .status();

    if which("rjq") {
        return Ok(None);
    }
    // The real bash if/elif structure: PATH is prepended unconditionally
    // whenever the bootstrap call succeeds with non-empty output, with NO
    // re-verification that rjq is then actually found on the newly-extended
    // PATH. Only the elif branch (the call itself failing, or succeeding
    // with empty output) is the failure path.
    let output = Command::new(repo_root.join("bootstrap.sh"))
        .args(["rjq", "--path-only"])
        .current_dir(repo_root)
        .output();
    let dir = output
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .filter(|dir| !dir.is_empty());
    match dir {
        Some(dir) => Ok(Some(dir)),
        None => {
            Err("run-tests.sh: rjq is still missing; the register tests cannot run.".to_string())
        }
    }
}

/// Builds the effective PATH string for a spawned child process, prepending
/// `extra_dir` (the rjq-fallback directory, when bootstrap_generated found
/// one) ahead of this process's own current PATH.
pub fn effective_path(extra_dir: Option<&str>) -> Option<String> {
    let extra_dir = extra_dir?;
    let existing = env::var("PATH").unwrap_or_default();
    Some(format!("{extra_dir}:{existing}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn scratch(tag: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("run-tests-bootstrap-{tag}-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn no_started_marker_is_clean() {
        let dir = scratch("no-marker");
        assert!(refuse_if_dev_env_dirty(&dir).is_ok());
    }

    #[test]
    fn matching_started_and_finished_tokens_are_clean() {
        let dir = scratch("match");
        fs::write(dir.join(".setup-dev-env.started"), "tok1\n").unwrap();
        fs::write(dir.join(".setup-dev-env.finished"), "tok1\n").unwrap();
        assert!(refuse_if_dev_env_dirty(&dir).is_ok());
    }

    #[test]
    fn mismatched_tokens_are_dirty() {
        let dir = scratch("mismatch");
        fs::write(dir.join(".setup-dev-env.started"), "tok1\n").unwrap();
        fs::write(dir.join(".setup-dev-env.finished"), "tok2\n").unwrap();
        let err = refuse_if_dev_env_dirty(&dir).unwrap_err();
        assert!(err.contains("a setup-dev-env.sh run started and never finished"));
    }

    #[test]
    fn started_with_no_finished_at_all_is_dirty() {
        let dir = scratch("no-finished");
        fs::write(dir.join(".setup-dev-env.started"), "tok1\n").unwrap();
        assert!(refuse_if_dev_env_dirty(&dir).is_err());
    }
}
