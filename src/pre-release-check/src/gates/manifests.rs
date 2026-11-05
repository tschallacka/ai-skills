// MODE: DEV
// PACKAGE: PROD

//! The manifest/marker/schema tests this release already depends on, each
//! already the authoritative implementation -- invoked here, never
//! reimplemented.

use crate::platform::script_command;
use crate::report::Report;
use std::path::Path;

fn run_test(repo_root: &Path, script: &str, ok_label: &str, bad_label: &str, report: &mut Report) {
    let status = script_command(&repo_root.join(script))
        .current_dir(repo_root)
        .status();
    match status {
        Ok(s) if s.success() => report.ok(ok_label),
        _ => report.bad(bad_label),
    }
}

pub fn gate_manifests(repo_root: &Path, report: &mut Report) {
    run_test(
        repo_root,
        "tests/test-skill-files-manifest.sh",
        "every tracked skill file is declared in skill_files()",
        "a skill file is tracked but not declared in skill_files() -- see tests/test-skill-files-manifest.sh",
        report,
    );
    run_test(
        repo_root,
        "tests/test-mode-markers.sh",
        "every scanned file carries a correct MODE/PACKAGE marker",
        "a scanned file's MODE/PACKAGE marker is missing or wrong -- see tests/test-mode-markers.sh",
        report,
    );
    run_test(
        repo_root,
        "tests/test-register-schemas.sh",
        "bug-report/todo/decisions each ship a schema for the installed version",
        "a register skill is missing schema.<version>.json, or its example/recipe is broken -- see tests/test-register-schemas.sh",
        report,
    );
}
