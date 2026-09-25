// MODE: DEV
// PACKAGE: PROD

//! The generated shell artifacts. The plan libraries rebuild whenever
//! `build-plan-libs.sh --check` itself judges them stale against their own
//! sources (B371: a bare existence check let a checkout install ship a
//! compiled library that predated a source change, since the generated
//! files are gitignored and a pull that changes a lib/ source leaves the
//! old compiled output sitting there, present but stale); REVIEWER.md and
//! PORTABILITY.md regenerate unconditionally every run, since neither
//! target exposes its own freshness check and both are cheap to rebuild.
//! Thin subprocess calls into the real scripts, never reimplemented. Also
//! wires the pre-push git hook via `git config core.hooksPath hooks`, the
//! one legitimate git invocation anywhere in this crate.
//!
//! Every subprocess spawned here carries `AI_SKILLS_BIN_ROOT=bin_dir` (B376):
//! `plan_bin_dir` (planning/scripts/plan-crypt-lib.sh) prefers a real
//! installed skill set at `~/.config/tsch-ai-skills/bin` over this repo's
//! own `bin/<triple>/` the moment that shared directory exists, and none of
//! these three tools (build-plan-libs, generate-reviewer,
//! generate-portability) is a binary any skill ships -- so on a machine
//! that already has skills installed, an unset override left every one of
//! them unable to find the binary this very run just built, even though it
//! sits right there in `bin_dir`.

use crate::platform::script_command;
use std::path::Path;
use std::process::{Command, Stdio};

/// `Command` inherits the parent's stdio by default, so each spawn must
/// explicitly discard both streams to keep them silent (found during goal
/// 17's own regression sweep: an earlier version let a spawned script's own
/// output line leak through to this crate's own stdout).
fn silent(mut command: Command) -> Command {
    command.stdout(Stdio::null()).stderr(Stdio::null());
    command
}

/// Every subprocess this module spawns needs the same override, so it is
/// applied here in one place rather than repeated at each call site.
fn with_bin_root(mut command: Command, bin_dir: &Path) -> Command {
    command.env("AI_SKILLS_BIN_ROOT", bin_dir);
    command
}

const LIBS: [&str; 5] = [
    "plan-core-lib.sh",
    "plan-crypt-lib.sh",
    "plan-document-lib.sh",
    "plan-progress-lib.sh",
    "plan-table-lib.sh",
];

/// Returns how many generated artifacts were (re)built this run -- used
/// only to decide whether to print "generated artifacts already present"
/// for the plan libraries specifically; REVIEWER.md and PORTABILITY.md each
/// report their own status unconditionally below, since both now always run.
pub fn build_if_missing(repo_root: &Path, bin_dir: &Path) -> u32 {
    let mut generated = 0;
    let libs_missing = LIBS
        .iter()
        .any(|lib| !repo_root.join("planning/scripts").join(lib).is_file());
    let libs_stale = libs_missing || {
        let mut check = with_bin_root(
            silent(script_command(
                &repo_root.join("planning/scripts/build-plan-libs.sh"),
            )),
            bin_dir,
        );
        !check
            .arg("--check")
            .current_dir(repo_root)
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
    };
    if libs_stale {
        if with_bin_root(
            silent(script_command(
                &repo_root.join("planning/scripts/build-plan-libs.sh"),
            )),
            bin_dir,
        )
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
    if with_bin_root(
        silent(script_command(
            &repo_root.join("planning/scripts/generate-reviewer.sh"),
        )),
        bin_dir,
    )
    .current_dir(repo_root)
    .status()
    .map(|s| s.success())
    .unwrap_or(false)
    {
        println!("setup-dev-env: regenerated planning/REVIEWER.md");
    } else {
        eprintln!("setup-dev-env: generate-reviewer.sh failed; the reviewer contract may be stale");
    }
    if with_bin_root(
        silent(script_command(&repo_root.join("generate-portability.sh"))),
        bin_dir,
    )
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
