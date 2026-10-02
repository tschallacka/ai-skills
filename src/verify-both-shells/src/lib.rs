// MODE: DEV
// PACKAGE: PROD
//! verify-both-shells — run the suite on the working tree under both
//! shells: a linked, detached git worktree; a stale-worktree liveness
//! sweep; a tracked-file overlay; two real shell-leg subprocess runs; a
//! content-based pass/fail report; and signal-safe cleanup.
//!
//! Exposed as a library, not only a binary, so integration tests can drive
//! `run()` directly with fake `Leg` values instead of needing a real
//! `flake.nix`/`nix develop` invocation inside the fast test suite.

pub mod git;
pub mod overlay;
pub mod platform;
pub mod process;
pub mod report;
pub mod signal;
pub mod sweep;
pub mod worktree;

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex};

pub const PROGRAM: &str = "verify-both-shells.sh";

pub const USAGE: &str =
    "verify-both-shells.sh — run the suite on the working tree under both shells.

Verifies the WORKING TREE, not HEAD, in a linked worktree away from the repo, so
editing can continue here while it runs. Both legs matter: the local bash, and
the bash 3.2 floor CODE-STYLE.md section 1 declares, because stock macOS ships
3.2 and a bash-4 construct is invisible under a newer shell.

Usage:
  ./verify-both-shells.sh            # both legs
  ./verify-both-shells.sh --keep     # keep the logs and the worktree on failure
  ./verify-both-shells.sh --help

A linked worktree rather than a clone: it shares the object store, so every ref
is present. blast-radius.sh resolves a merge base against master, and a clone
has only origin/master -- which made an earlier version of this report a failure
of itself as a failure of the code. --detach because the branch is checked out
in the main repo, and it lives in TMPDIR, never under the repo: a worktree
inside it gets picked up by the filesystem scans and lands machine-specific
paths in generated artifacts, which is how PORTABILITY.md was once polluted.

";

pub enum Action {
    Help,
    Run { keep: bool },
}

pub fn parse_args(args: &[String]) -> Result<Action, i32> {
    match args.first().map(|s| s.as_str()) {
        None => Ok(Action::Run { keep: false }),
        Some("--keep") => Ok(Action::Run { keep: true }),
        Some("-h") | Some("--help") => Ok(Action::Help),
        Some(other) => {
            eprintln!("{PROGRAM}: unknown argument: {other}");
            Err(64)
        }
    }
}

/// PLANNING_SKILL_ROOT first, current_exe()-anchored ancestor search as
/// fallback -- never `git rev-parse`.
pub fn discover_repo_root() -> Result<PathBuf, String> {
    if let Ok(root) = env::var("PLANNING_SKILL_ROOT") {
        if !root.is_empty() {
            return Ok(PathBuf::from(root));
        }
    }
    let self_path = env::current_exe().unwrap_or_else(|_| PathBuf::from("verify-both-shells"));
    let mut dir = self_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    loop {
        if dir.join("planning/scripts").is_dir() {
            return Ok(dir);
        }
        let Some(parent) = dir.parent() else {
            return Err(format!(
                "could not locate the repository root from {}",
                self_path.display()
            ));
        };
        dir = parent.to_path_buf();
    }
}

/// Cleanup state a signal handler and the normal-path end of `run()` both
/// observe and act on identically.
struct Shared {
    src: PathBuf,
    parent: Mutex<Option<PathBuf>>,
    wt: Mutex<Option<PathBuf>>,
    log5: Mutex<Option<PathBuf>>,
    log3: Mutex<Option<PathBuf>>,
    keep: bool,
    status: AtomicI32,
}

/// Cleanup order: the worktree and its own parent directory (holding
/// `harness.pid`) are removed UNCONDITIONALLY, every time -- `--keep`
/// never affects them. Only AFTERWARD, separately, are the two independent
/// `log5`/`log3` temp files (created directly under `${TMPDIR:-/tmp}`, NOT
/// nested inside the worktree's own parent) either left alone (printing
/// "logs kept") when `keep && status != 0`, or deleted otherwise.
fn cleanup(shared: &Shared) {
    if let Some(wt) = shared.wt.lock().unwrap().take() {
        git::worktree_remove(&shared.src, &wt);
    }
    if let Some(parent) = shared.parent.lock().unwrap().take() {
        let _ = fs::remove_dir_all(&parent);
    }
    let log5 = shared.log5.lock().unwrap().take();
    let log3 = shared.log3.lock().unwrap().take();
    let keep_logs = shared.keep && shared.status.load(Ordering::SeqCst) != 0;
    if keep_logs {
        if let (Some(log5), Some(log3)) = (&log5, &log3) {
            eprintln!("logs kept: {} {}", log5.display(), log3.display());
        }
    } else {
        if let Some(log5) = &log5 {
            let _ = fs::remove_file(log5);
        }
        if let Some(log3) = &log3 {
            let _ = fs::remove_file(log3);
        }
    }
}

/// The exit status, plus the two real leg log-file paths this run used (if
/// it got far enough to create them) -- exposed mainly so tests can check
/// the real `--keep` behavior directly instead of needing to intercept
/// `run()`'s own `eprintln!`.
pub struct RunOutcome {
    pub status: i32,
    pub log5: Option<PathBuf>,
    pub log3: Option<PathBuf>,
}

pub fn run(src: &Path, keep: bool, legs: [process::Leg; 2]) -> RunOutcome {
    let base = platform::tmp_base();
    let Some(parent) = worktree::mktemp_scratch_dir(&base) else {
        eprintln!("{PROGRAM}: mktemp -d failed");
        return RunOutcome {
            status: 70,
            log5: None,
            log3: None,
        };
    };
    let wt = parent.join("tree");
    let _ = fs::write(parent.join("harness.pid"), std::process::id().to_string());

    let log5 = worktree::mktemp_file(&base, "verify-log5");
    let log3 = worktree::mktemp_file(&base, "verify-log3");

    let shared = Arc::new(Shared {
        src: src.to_path_buf(),
        parent: Mutex::new(Some(parent.clone())),
        wt: Mutex::new(None),
        log5: Mutex::new(log5.clone()),
        log3: Mutex::new(log3.clone()),
        keep,
        status: AtomicI32::new(0),
    });

    let signal_shared = Arc::clone(&shared);
    signal::install_signal_cleanup(move || cleanup(&signal_shared));

    sweep::sweep_stale_worktrees(src, &wt);

    if !git::worktree_add(src, &wt) {
        cleanup(&shared);
        return RunOutcome {
            status: 70,
            log5,
            log3,
        };
    }
    *shared.wt.lock().unwrap() = Some(wt.clone());

    let overlaid = overlay::overlay(src, &wt);
    println!(
        "worktree {} (base {}, {} file(s) overlaid)",
        wt.display(),
        git::rev_parse_short_head(src),
        overlaid
    );

    let (Some(log5), Some(log3)) = (log5.clone(), log3.clone()) else {
        eprintln!("{PROGRAM}: mktemp failed");
        cleanup(&shared);
        return RunOutcome {
            status: 70,
            log5,
            log3,
        };
    };

    let results: Vec<report::ReportResult> = legs
        .iter()
        .zip([&log5, &log3])
        .map(|(leg, log)| {
            if let Some(reason) = leg.skip_reason {
                return report::skipped(leg.label, reason);
            }
            let mut command = (leg.build)(&wt);
            let _ = process::run_leg_to_file(&mut command, log);
            let text = fs::read_to_string(log).unwrap_or_default();
            report::report(leg.label, &text)
        })
        .collect();
    let (r5, r3) = (&results[0], &results[1]);
    print!("{}", r5.text);
    print!("{}", r3.text);
    let status_value = if r5.is_failure || r3.is_failure { 1 } else { 0 };
    shared.status.store(status_value, Ordering::SeqCst);

    cleanup(&shared);
    RunOutcome {
        status: status_value,
        log5: Some(log5),
        log3: Some(log3),
    }
}
