// MODE: DEV
// PACKAGE: PROD

//! Gate 1 (whitespace) and gate 1b (the portability catalogue).

use crate::report::Report;
use std::path::Path;
use std::process::Command;

fn git_status_ok(repo_root: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false)
}

pub fn gate_whitespace(repo_root: &Path, base: Option<&str>, report: &mut Report) {
    let worktree_ok = git_status_ok(repo_root, &["diff", "--check"]);
    let index_ok = git_status_ok(repo_root, &["diff", "--cached", "--check"]);
    let branch_ok = match base {
        Some(base) => {
            let range = format!("{base}..HEAD");
            git_status_ok(repo_root, &["diff", "--check", &range])
        }
        None => true,
    };
    if worktree_ok && index_ok && branch_ok {
        report.ok("git diff --check (worktree, index, branch diff)");
    } else {
        report.bad("whitespace errors: git diff --check");
    }
}

/// Unconditional, every run: PORTABILITY.md is untracked and cheap to
/// rebuild, so a push always leaves the working tree with a copy that
/// actually matches what just got pushed.
pub fn gate_portability(repo_root: &Path, report: &mut Report) {
    let script = repo_root.join("generate-portability.sh");
    let ok = Command::new(&script)
        .current_dir(repo_root)
        .output()
        .map(|out| out.status.success())
        .unwrap_or(false);
    if ok {
        report.ok("regenerated PORTABILITY.md");
    } else {
        report.bad("generate-portability.sh failed; the portability catalogue may be stale");
    }
}
