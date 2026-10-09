// MODE: DEV
// PACKAGE: PROD

//! RELEASE.md step 4: every generated artifact confirmed fresh, each via
//! its own `--check` (or, for blast-radius, its own always-checking run).

use crate::platform::script_command;
use crate::report::Report;
use std::path::Path;

fn run_check(repo_root: &Path, script: &str, args: &[&str], ok_label: &str, report: &mut Report) {
    let status = script_command(&repo_root.join(script))
        .args(args)
        .current_dir(repo_root)
        .status();
    match status {
        Ok(s) if s.success() => report.ok(ok_label),
        _ => report.bad(&format!(
            "{script} {} failed -- run it directly and inspect",
            args.join(" ")
        )),
    }
}

pub fn gate_generated_artifacts(repo_root: &Path, report: &mut Report) {
    run_check(
        repo_root,
        "planning/scripts/build-plan-libs.sh",
        &["--check"],
        "build-plan-libs.sh --check: the five plan-*-lib.sh are fresh",
        report,
    );
    run_check(
        repo_root,
        "generate-portability.sh",
        &["--check"],
        "generate-portability.sh --check: PORTABILITY.md is fresh",
        report,
    );
    run_check(
        repo_root,
        "blast-radius.sh",
        &[],
        "blast-radius.sh: every coupling in coupling.tsv",
        report,
    );
}
