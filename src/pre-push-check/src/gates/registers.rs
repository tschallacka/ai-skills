// MODE: DEV
// PACKAGE: PROD

//! Gate 0: BUGS.json and TODO.json are filed on the `registers` branch and
//! nowhere else, so ids are allocated in one place and no merge ever has to
//! reconcile two sets of them. Fails immediately, before any other gate
//! runs, with its own distinct terminal line (not the normal final summary
//! line).
//!
//! The other half is `registers` itself: a push from that branch is checked
//! for one thing only, that nothing but the two registers changed, and no
//! other gate runs (see `gate_register_branch_scope`).

use crate::change_set::{changed, changed_files, current_branch};
use crate::report::Report;
use std::env;
use std::path::Path;

const REGISTER_BRANCH: &str = "registers";
const REGISTER_FILES: [&str; 2] = ["BUGS.json", "TODO.json"];

pub fn on_register_branch(repo_root: &Path) -> bool {
    current_branch(repo_root) == REGISTER_BRANCH
}

/// The only gate a push from the `registers` branch runs: every changed path
/// must be BUGS.json or TODO.json. Register changes reach master without
/// review (.github/workflows/registers.yml), so a branch that could carry
/// anything else is a protection bypass; that CI guard is the authority and
/// this refuses the same thing before the push leaves the machine. Nothing
/// here needs cargo, a shell, or nix, so it runs on a host whose dev shell
/// cannot be built.
pub fn gate_register_branch_scope(repo_root: &Path, base: Option<&str>, report: &mut Report) {
    let files = changed_files(repo_root, base);
    let stray: Vec<&String> = files
        .iter()
        .filter(|f| !REGISTER_FILES.contains(&f.as_str()))
        .collect();
    if stray.is_empty() {
        if files.is_empty() {
            report.note("nothing differs from master");
        } else {
            report.ok(&format!(
                "only registers changed on the {REGISTER_BRANCH} branch ({})",
                files.join(", ")
            ));
        }
        report.note("registers branch: no other gate runs; registers.yml checks ids and parents when it lands");
        return;
    }
    report.bad(&format!(
        "the {REGISTER_BRANCH} branch may only change BUGS.json and TODO.json"
    ));
    for file in stray {
        println!("    {file}");
    }
    report.note("register changes reach master without review, so anything else is refused");
    report.note(
        "put the other change on its own branch: git switch -c <name> from the commit before it",
    );
}

pub fn gate_registers_branch(
    repo_root: &Path,
    base: Option<&str>,
    report: &mut Report,
) -> Result<(), i32> {
    let mut register_changes = changed(repo_root, base, r"^(BUGS|TODO)\.json$");
    let branch = current_branch(repo_root);

    // PRE_PUSH_ALLOW_REGISTERS=1 is for transport, not authoring: never used
    // to file an entry, only to accept register changes already in flight.
    if env::var("PRE_PUSH_ALLOW_REGISTERS").as_deref() == Ok("1") && !register_changes.is_empty() {
        report.note(&format!(
            "PRE_PUSH_ALLOW_REGISTERS=1: register changes accepted off the {REGISTER_BRANCH} branch"
        ));
        register_changes.clear();
    }

    if !register_changes.is_empty() && branch != REGISTER_BRANCH {
        report.bad(&format!(
            "a register is modified outside the {REGISTER_BRANCH} branch"
        ));
        for file in &register_changes {
            println!("    {file}");
        }
        report.note(&format!("branch: {branch}"));
        report.note(&format!("THE TARGET BRANCH IS: {REGISTER_BRANCH}"));
        report.note(&format!(
            "  git switch {REGISTER_BRANCH}   (git switch -c {REGISTER_BRANCH} origin/{REGISTER_BRANCH} if it is not local yet)"
        ));
        report.note("  then file the entry with the shipped tools -- bin/<triple>/bugs add ... or");
        report.note("  bin/<triple>/todo add ... -- and push; the entry reaches master from there");
        report
            .note("a fix's resolution keys (fix, verification, status) go the same way, after the");
        report.note("  code lands, so the id is allocated and closed in one place");
        println!(
            "pre-push-check: 1 failure(s) - registers changed off the {REGISTER_BRANCH} branch"
        );
        return Err(1);
    }
    Ok(())
}
