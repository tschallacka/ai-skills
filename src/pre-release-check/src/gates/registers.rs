// MODE: DEV
// PACKAGE: PROD

//! The repo root's own BUGS.json/TODO.json/DECISIONS.json (what
//! planning/tests/test-register-helpers.sh and friends seed their fixtures
//! from) must already carry the current skill_version, or every test that
//! copies them and then calls the freshly-bumped CLI binary refuses --
//! confirmed the hard way this release, after the registers worktree's own
//! migrate had not yet been pushed and forwarded to master.
//!
//! The dedicated registers worktree itself is environment-dependent (it may
//! not exist on a CI runner at all), so its own checks are notes, not
//! failures; a missing worktree is not this release's problem to fix.

use crate::report::Report;
use std::path::Path;
use std::process::Command;

const REGISTERS: &[(&str, &str)] = &[
    ("BUGS.json", "bugs"),
    ("TODO.json", "todo"),
    ("DECISIONS.json", "decisions"),
];

fn skill_version_of(path: &Path) -> Option<String> {
    let text = std::fs::read_to_string(path).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    value
        .get("skill_version")
        .and_then(|v| v.as_str())
        .map(str::to_string)
}

pub fn gate_repo_root_registers(repo_root: &Path, package_version: &str, report: &mut Report) {
    for (file, label) in REGISTERS {
        let path = repo_root.join(file);
        if !path.is_file() {
            report.note(&format!("{file}: not present at the repo root; skipped"));
            continue;
        }
        match skill_version_of(&path) {
            Some(v) if v == package_version => {
                report.ok(&format!("{file}: skill_version matches package.json ({v})"))
            }
            Some(v) => report.bad(&format!(
                "{file}: skill_version is {v}, package.json is {package_version} -- `{label} migrate` it on the registers branch, then push and let it forward to master"
            )),
            None => report.bad(&format!("{file}: could not read its own skill_version field")),
        }
    }
}

/// Every git worktree checked out on `registers`, found via `git worktree
/// list` rather than either of this project's two documented candidate
/// paths (B404's own finding: both exist in the wild) -- so this notes
/// whichever one is actually there, on any machine.
pub fn note_registers_worktree(repo_root: &Path, report: &mut Report) {
    let output = Command::new("git")
        .args(["worktree", "list", "--porcelain"])
        .current_dir(repo_root)
        .output();
    let Ok(output) = output else {
        report.note("git worktree list failed; cannot locate the registers worktree");
        return;
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let mut worktree_path: Option<&str> = None;
    let mut path = "";
    for line in text.lines() {
        if let Some(p) = line.strip_prefix("worktree ") {
            path = p;
        } else if line == "branch refs/heads/registers" {
            worktree_path = Some(path);
        }
    }
    match worktree_path {
        Some(path) => report.note(&format!("registers worktree found at {path}")),
        None => {
            report.note("no git worktree is checked out on the registers branch on this machine")
        }
    }
}
