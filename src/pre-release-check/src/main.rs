// MODE: DEV
// PACKAGE: PROD

//! Everything RELEASE.md's checklist and this repository's own packaging
//! machinery need checked before a release is tagged, automated wherever a
//! gate can decide the answer on its own, and a printed CHECKLIST for every
//! part that genuinely needs a human or an agent's own judgement.
//!
//! Born directly from the 2.0.0-alpha.6 release: every gate here exists
//! because something in that specific pass went wrong silently and was
//! only caught by hand -- see each gate module's own doc comment for which
//! one.

mod checklist;
mod gates;
mod platform;
mod reexec;
mod repo_root;
mod report;

use gates::full_suite::gate_full_suite;
use gates::generated::gate_generated_artifacts;
use gates::manifests::gate_manifests;
use gates::plugin_parity::gate_plugin_parity;
use gates::registers::{gate_repo_root_registers, note_registers_worktree};
use gates::release_artifacts::gate_release_artifacts;
use gates::version::{
    gate_skill_md_examples, gate_version_consistency, note_stale_version_literals,
};
use report::Report;
use std::path::PathBuf;
use std::process::ExitCode;

const PROGRAM: &str = "pre-release-check.sh";

const USAGE: &str = r#"pre-release-check - everything RELEASE.md's checklist needs verified before
tagging a release, in one command, plus a printed checklist for what it
cannot verify on its own.

Usage:
  pre-release-check.sh           the gates below
  pre-release-check.sh --full    also run ./run-tests.sh (the whole suite,
                                  under the resource wrapper)
  pre-release-check.sh --help

Gates, in order: each register skill's own SUPPORTED constant against
package.json's version; each register skill's own SKILL.md worked example
against the same version (a hard failure -- a worked example has no
legitimate reason to lag); a scan for test fixtures still naming an older
skill_version (printed as notes, not failures -- see the gate's own doc
comment for why); the generated artifacts RELEASE.md step 4 names
(build-plan-libs.sh, generate-portability.sh, blast-radius.sh); the manifest
and marker tests (test-skill-files-manifest.sh, test-mode-markers.sh,
test-register-schemas.sh); the five vendor plugins' own file lists, cross-
checked against installer/build-release.sh, src/installer/src/plugins.rs,
and a real `npm pack --dry-run` (the class of bug this tool exists for: two
real, silent packaging gaps found only by a from-scratch audit); the release
tarball and npm package tests; and the repo root's own BUGS.json/TODO.json/
DECISIONS.json skill_version.

Exit codes: 0 = every gate passed; 1 = at least one gate failed; 64 = bad
usage; 65 = not a git repository; 69 = nix is needed to re-enter the
development shell and is not on PATH, so NO gate ran; 70 = package.json
could not be read or has no version.
"#;

enum ParseOutcome {
    Run { full: bool },
    Exit(u8),
}

fn parse_args(args: &[String]) -> ParseOutcome {
    let mut full = false;
    for arg in args {
        match arg.as_str() {
            "--full" => full = true,
            "-h" | "--help" => {
                print!("{USAGE}");
                return ParseOutcome::Exit(0);
            }
            other => {
                eprintln!("{PROGRAM}: unknown argument: {other}");
                print!("{USAGE}");
                return ParseOutcome::Exit(64);
            }
        }
    }
    ParseOutcome::Run { full }
}

fn package_version(repo_root: &std::path::Path) -> Option<String> {
    let text = std::fs::read_to_string(repo_root.join("package.json")).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    value.get("version")?.as_str().map(str::to_string)
}

fn finish(report: &Report) -> ExitCode {
    if report.failures == 0 {
        println!("pre-release-check: PASS");
        checklist::print_checklist();
        ExitCode::SUCCESS
    } else {
        println!("pre-release-check: {} failure(s)", report.failures);
        checklist::print_checklist();
        ExitCode::FAILURE
    }
}

fn run() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    let reexec_root = repo_root::discover_repo_root().unwrap_or_else(|| PathBuf::from("."));
    reexec::maybe_reexec(&reexec_root);

    let full = match parse_args(&args) {
        ParseOutcome::Exit(code) => return ExitCode::from(code),
        ParseOutcome::Run { full } => full,
    };

    let Some(repo_root) = repo_root::discover_repo_root() else {
        eprintln!("{PROGRAM}: not a git repository");
        return ExitCode::from(65);
    };
    let Some(version) = package_version(&repo_root) else {
        eprintln!("{PROGRAM}: could not read package.json's own version");
        return ExitCode::from(70);
    };

    println!("pre-release-check (package.json version: {version})");
    let mut report = Report::new();

    gate_version_consistency(&repo_root, &version, &mut report);
    gate_skill_md_examples(&repo_root, &version, &mut report);
    note_stale_version_literals(&repo_root, &version, &mut report);
    gate_generated_artifacts(&repo_root, &mut report);
    gate_manifests(&repo_root, &mut report);
    gate_plugin_parity(&repo_root, &mut report);
    gate_release_artifacts(&repo_root, &mut report);
    gate_repo_root_registers(&repo_root, &version, &mut report);
    note_registers_worktree(&repo_root, &mut report);
    gate_full_suite(&repo_root, full, &mut report);

    finish(&report)
}

fn main() -> ExitCode {
    run()
}
