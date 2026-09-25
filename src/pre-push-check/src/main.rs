// MODE: DEV
// PACKAGE: PROD
mod change_set;
mod gates;
mod platform;
mod reexec;
mod report;

use change_set::{fetch_master, resolve_base};
use gates::full_suite::gate_full_suite;
use gates::manifest::gate_skill_manifest;
use gates::npm_baseline::gate_npm_baseline;
use gates::register_soundness::gate_register_soundness;
use gates::registers::{gate_register_branch_scope, gate_registers_branch, on_register_branch};
use gates::rust_crates::gate_rust_crates;
use gates::shellcheck::gate_shellcheck;
use gates::static_scans::gate_static_scans;
use gates::syntax::gate_bash_syntax;
use gates::whitespace::{gate_portability, gate_whitespace};
use report::Report;
use std::path::PathBuf;
use std::process::{Command, ExitCode};

// The literal script file name, not the compiled binary's own bare name.
const PROGRAM: &str = "pre-push-check.sh";

// The usage text, embedded here as a literal constant rather than derived
// at runtime.
const USAGE: &str = r#"pre-push-check - the per-change gates and the PR hygiene rules in AGENTS.md,
in one command.

Run it before every push. It re-enters `nix develop` first, so without nix it
exits 69 and runs no gate, --help included (the `registers` branch is exempt,
see the paragraph after the gate list). The change set is everything that
differs from the merge base with origin/master (falling back to master, then
to the branch's upstream) - the branch's commits plus the worktree and the
index - and the gates run in this order:
  git fetch origin master  refreshes master before anything is measured, and
                          the base is resolved from what it fetched; a failed
                          fetch ends the run (PRE_PUSH_SKIP_FETCH=1 skips it,
                          for a throwaway clone or no network, and the change
                          set may then be stale)
  registers-branch guard  BUGS.json or TODO.json changed on any branch but
                          `registers` is refused and ends the run
                          (PRE_PUSH_ALLOW_REGISTERS=1 accepts changes already
                          in flight; never use it to file an entry)
  git diff --check        whitespace, in the worktree, the index and the
                          branch's committed diff
  PORTABILITY.md          regenerated unconditionally, so it always matches
                          what's about to be pushed (it is untracked; see
                          generate-portability.sh)
  bash -n                 every changed shell script
  static shell gate       the changed scripts at warning severity, with -x so
                          `source=` resolves from disk. CI lints the whole
                          live set; see the note at that gate for why the
                          two agree and where they cannot
  static scans            a function newly over CODE-STYLE.md's 40-line cap,
                          and a construct PORTABILITY.md bans in a changed
                          script
  cargo fmt --check +     each crate under src/ touched by the change
  cargo test              (skipped with a note when no crate changed)
  cargo clippy            the whole workspace, --all-targets -D warnings,
                          whenever any crate changed
  register soundness      TODO.json and BUGS.json through reg_findings, the
                          shipped implementation: ids, statuses, severities,
                          priorities, parents, timestamps, reproductions,
                          mechanism-on-confirmed, verification-on-fixed
                          (needs rjq on PATH)
  npm package baseline    every pinned byte size in
                          planning/tests/fixtures/overview/npm-package-baseline.tsv
                          against the working tree. Not npm's file selection -
                          the full planning/tests/test-npm-package.sh still
                          owns that
  skill_files() check     tests/test-skill-files-manifest.sh
                          --declarations-only: a tracked skill file that
                          installer/src/50-manifest.sh's skill_files() does
                          not declare fails
On the `registers` branch none of the above runs, and neither does the nix
re-entry: the one gate is that every changed path is BUGS.json or TODO.json,
and anything else fails. The registers
workflow (.github/workflows/registers.yml) checks ids and parents when the
push lands.
The registers update, the plan validator and the role-drift tests stay with
.agents/MAINTAINER.md section 2 (and planning/MAINTAINER.md section 4 for a
change to the planning skill): they need judgement about what changed, which a
pre-push helper deliberately does not guess at.

Usage:
  pre-push-check.sh           the gates above
  pre-push-check.sh --full    also run ./run-tests.sh (the whole suite,
                              under the resource wrapper)
  pre-push-check.sh --help

Exit codes: 0 = every gate passed; 1 = at least one gate failed; 64 = bad
usage; 65 = not a git repository, or neither master nor an upstream resolves
and there is no branch diff to check; 69 = nix is needed to re-enter the
development shell and is not on PATH, so NO gate ran. 69 was undocumented,
and a caller that read only "non-zero" therefore reported a refusal for a run
that never started -- which is exactly what tests/test-register-branch-gate.sh
did on the macOS bash 3.2 leg, where there is no nix.
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

fn discover_repo_root() -> Option<PathBuf> {
    let output = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (!text.is_empty()).then(|| PathBuf::from(text))
}

/// The verdict line and exit code every path ends on.
fn finish(report: &Report) -> ExitCode {
    if report.failures == 0 {
        println!("pre-push-check: PASS");
        ExitCode::SUCCESS
    } else {
        println!("pre-push-check: {} failure(s)", report.failures);
        ExitCode::FAILURE
    }
}

fn run() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();

    // The nix re-exec runs before repo-root discovery: it needs no git
    // context of its own, only the flake at the repo root the re-exec'd
    // process is told to develop against. The `registers` branch skips it:
    // its one gate needs only git, and its flake may be the stale master one
    // whose dev shell does not build on every host (B333).
    let reexec_root = discover_repo_root().unwrap_or_else(|| PathBuf::from("."));
    let on_registers = on_register_branch(&reexec_root);
    if !on_registers {
        reexec::maybe_reexec(&reexec_root);
    }

    let full = match parse_args(&args) {
        ParseOutcome::Exit(code) => return ExitCode::from(code),
        ParseOutcome::Run { full } => full,
    };

    let Some(repo_root) = discover_repo_root() else {
        eprintln!("{PROGRAM}: not a git repository");
        return ExitCode::from(65);
    };
    if std::env::set_current_dir(&repo_root).is_err() {
        eprintln!("{PROGRAM}: not a git repository");
        return ExitCode::from(65);
    }

    // master is the branch every change set is measured against, so it is
    // refreshed before anything else, and the base is resolved from what that
    // fetch brought in. Resolving first would measure against a stale
    // origin/master and list master's own newer commits as this branch's.
    let mut report = Report::new();
    if let Err(code) = fetch_master(&repo_root, &mut report) {
        return ExitCode::from(code as u8);
    }
    let resolved = resolve_base(&repo_root);
    let base = resolved.base.as_deref();
    let base_label = resolved.label.as_deref();

    println!(
        "pre-push-check (base: {}{})",
        base_label.unwrap_or("no master or upstream; worktree only"),
        if on_registers {
            "; registers branch"
        } else {
            ""
        }
    );

    if on_registers {
        gate_register_branch_scope(&repo_root, base, &mut report);
        return finish(&report);
    }

    if let Err(code) = gate_registers_branch(&repo_root, base, &mut report) {
        return ExitCode::from(code as u8);
    }

    gate_whitespace(&repo_root, base, &mut report);
    gate_portability(&repo_root, &mut report);
    gate_bash_syntax(&repo_root, base, &mut report);

    let changed_sh = gate_shellcheck(&repo_root, base, base_label, &mut report);
    gate_static_scans(&repo_root, base, base_label, &changed_sh, &mut report);
    gate_rust_crates(&repo_root, base, &mut report);

    gate_register_soundness(&repo_root, &mut report);
    gate_npm_baseline(&repo_root, &mut report);
    gate_skill_manifest(&repo_root, &mut report);
    gate_full_suite(&repo_root, full, &mut report);

    finish(&report)
}

fn main() -> ExitCode {
    run()
}
