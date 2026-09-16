// MODE: DEV
// PACKAGE: PROD

//! Gate 0: BUGS.json and TODO.json are filed on the `registers` branch and
//! nowhere else, so ids are allocated in one place and no merge ever has to
//! reconcile two sets of them. Fails immediately, before any other gate runs
//! -- mirrors pre-push-check.sh's own early `exit 1` exactly, including its
//! own distinct terminal line (not the normal final summary line).

use crate::change_set::{changed, current_branch};
use crate::report::Report;
use std::env;
use std::path::Path;

const REGISTER_BRANCH: &str = "registers";

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
            "  git switch {REGISTER_BRANCH}   (git switch -c {REGISTER_BRANCH} origin/master if it is not local yet)"
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
