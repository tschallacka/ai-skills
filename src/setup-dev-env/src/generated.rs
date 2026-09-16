// MODE: DEV
// PACKAGE: PROD

//! The generated shell artifacts, on the same build-if-missing terms as
//! the crates: build-plan-libs.sh and generate-reviewer.sh regenerate only
//! when their own output is missing; generate-portability.sh regenerates
//! unconditionally, every run. Matches run-tests's own bootstrap_generated
//! precedent -- thin subprocess calls into the real bash scripts, never
//! reimplemented. Also wires the pre-push git hook via `git config
//! core.hooksPath hooks`, the one legitimate git invocation anywhere in
//! this crate.

use std::path::Path;
use std::process::{Command, Stdio};

/// Bash's own three calls here are each `... >/dev/null 2>&1`; `Command`
/// inherits the parent's stdio by default, so each spawn must explicitly
/// discard both streams to match (found during goal 17's own regression
/// sweep: an earlier version let generate-portability.sh's own "Wrote ..."
/// line leak through to setup-dev-env's own stdout).
fn silent(mut command: Command) -> Command {
    command.stdout(Stdio::null()).stderr(Stdio::null());
    command
}

const LIBS: [&str; 5] = [
    "plan-core-lib.sh",
    "plan-crypt-lib.sh",
    "plan-document-lib.sh",
    "plan-progress-lib.sh",
    "plan-table-lib.sh",
];

/// Returns how many generated artifacts were (re)built this run, matching
/// bash's own `generated` counter (used only to decide whether to print
/// "generated artifacts already present").
pub fn build_if_missing(repo_root: &Path) -> u32 {
    let mut generated = 0;
    let missing_lib = LIBS
        .iter()
        .any(|lib| !repo_root.join("planning/scripts").join(lib).is_file());
    if missing_lib {
        if silent(Command::new(
            repo_root.join("planning/scripts/build-plan-libs.sh"),
        ))
        .current_dir(repo_root)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
        {
            println!("setup-dev-env: built the generated shell libraries");
            generated += 1;
        } else {
            eprintln!("setup-dev-env: build-plan-libs.sh failed; planning helpers will not load");
        }
    }
    if !repo_root.join("planning/REVIEWER.md").is_file() {
        if silent(Command::new(
            repo_root.join("planning/scripts/generate-reviewer.sh"),
        ))
        .current_dir(repo_root)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
        {
            println!("setup-dev-env: generated planning/REVIEWER.md");
            generated += 1;
        } else {
            eprintln!(
                "setup-dev-env: generate-reviewer.sh failed; the reviewer contract is missing"
            );
        }
    }
    if silent(Command::new(repo_root.join("generate-portability.sh")))
        .current_dir(repo_root)
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        println!("setup-dev-env: regenerated PORTABILITY.md");
    } else {
        eprintln!(
            "setup-dev-env: generate-portability.sh failed; the portability catalogue may be stale"
        );
    }
    if generated == 0 {
        println!("setup-dev-env: generated artifacts already present");
    }
    generated
}

pub fn wire_pre_push_hook(repo_root: &Path) {
    let ok = Command::new("git")
        .arg("-C")
        .arg(repo_root)
        .args(["config", "core.hooksPath", "hooks"])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if ok {
        println!("setup-dev-env: pre-push gate wired (git config core.hooksPath hooks)");
    } else {
        eprintln!("setup-dev-env: could not set core.hooksPath; the pre-push gate is not wired");
    }
}
