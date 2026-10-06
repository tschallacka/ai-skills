// MODE: DEV
// PACKAGE: PROD

//! refuse_if_dev_env_dirty and bootstrap_generated: the two pre-flight
//! checks performed before any test runs, both shelling out to the real
//! scripts rather than reimplementing their own logic.

use crate::platform::{self, script_command};
use std::path::Path;

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
        let _ = script_command(&repo_root.join("planning/scripts/build-plan-libs.sh"))
            .current_dir(repo_root)
            .status();
    }
    if !repo_root.join("planning/REVIEWER.md").is_file() {
        let _ = script_command(&repo_root.join("planning/scripts/generate-reviewer.sh"))
            .current_dir(repo_root)
            .status();
    }
    let _ = script_command(&repo_root.join("generate-portability.sh"))
        .current_dir(repo_root)
        .status();

    if shipped_rjq_present() {
        return Ok(None);
    }
    // The bootstrap call names the directory that holds rjq, and that directory
    // is handed to the children as AI_SKILLS_BIN_ROOT. It is never put on PATH:
    // rjq is a shipped tool, run by its path. Only a failing call, or one
    // succeeding with empty output, is treated as a failure.
    let output = script_command(&repo_root.join("bootstrap.sh"))
        .args(["rjq", "--path-only"])
        .current_dir(repo_root)
        .output();
    let dir = output
        .ok()
        .filter(|out| out.status.success())
        .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())
        .filter(|dir| !dir.is_empty())
        // A bash prints `/d/a/x`; the children PATH is handed to need `D:\a\x`.
        .map(|dir| platform::to_native_path(&dir));
    match dir {
        Some(dir) => Ok(Some(dir)),
        None => {
            Err("run-tests.sh: rjq is still missing; the register tests cannot run.".to_string())
        }
    }
}

/// Whether the shipped rjq is in the shared bin directory, or in the one
/// AI_SKILLS_BIN_ROOT names. Looked up by path, never through PATH.
pub fn shipped_rjq_present() -> bool {
    let mut dirs: Vec<std::path::PathBuf> = Vec::new();
    if let Some(root) = std::env::var_os("AI_SKILLS_BIN_ROOT") {
        dirs.push(root.into());
    }
    let config = std::env::var_os("XDG_CONFIG_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| std::path::PathBuf::from(home).join(".config")));
    if let Some(config) = config {
        dirs.push(config.join("tsch-ai-skills").join("bin"));
    }
    dirs.iter()
        .any(|dir| dir.join("rjq").is_file() || dir.join("rjq.exe").is_file())
}

/// The bin directory handed to the children as AI_SKILLS_BIN_ROOT, when the
/// bootstrap had to locate one. The children never see it on PATH.
pub fn bin_root_for_children(extra_dir: Option<&str>) -> Option<String> {
    extra_dir.map(str::to_string)
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
