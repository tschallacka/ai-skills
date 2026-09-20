// MODE: DEV
// PACKAGE: PROD

//! Gate 7: the whole suite, only when --full is given. Inherits stdio so
//! run-tests.sh's own output reaches the terminal directly, matching bash's
//! own unredirected `./run-tests.sh`.

use crate::platform::script_command;
use crate::report::Report;
use std::path::Path;

pub fn gate_full_suite(repo_root: &Path, full: bool, report: &mut Report) {
    if !full {
        report.note("the whole deterministic suite is ./run-tests.sh (or re-run with --full)");
        return;
    }
    let ok = script_command(&repo_root.join("run-tests.sh"))
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
