// MODE: DEV
// PACKAGE: PROD

//! The whole deterministic suite, only when --full is given, under the
//! resource wrapper -- mirrors pre-push-check's own gate_full_suite.

use crate::platform::script_command;
use crate::report::Report;
use std::path::Path;

pub fn gate_full_suite(repo_root: &Path, full: bool, report: &mut Report) {
    if !full {
        report.note("the whole deterministic suite is ./run-tests.sh (or re-run with --full)");
        return;
    }
    let ok = script_command(&repo_root.join("resource-limited-testing/scripts/limited-run.sh"))
        .args(["6G", "400", "--"])
        .arg(repo_root.join("run-tests.sh"))
        .current_dir(repo_root)
        .status()
        .map(|status| status.success())
        .unwrap_or(false);
    if ok {
        report.ok("run-tests.sh: the whole suite");
    } else {
        report.bad("run-tests.sh reported failures");
    }
}
