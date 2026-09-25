// MODE: DEV
// PACKAGE: PROD

//! Gate 2: bash -n on every changed shell script that still exists. A
//! deletion is part of the change set, and feeding a deleted path to bash -n
//! fails, so this only checks paths that still exist on disk -- matching the
//! bash original's own `[ -f "$f" ] || continue`.

use crate::change_set::changed;
use crate::platform::bash;
use crate::report::Report;
use std::path::Path;

pub fn gate_bash_syntax(repo_root: &Path, base: Option<&str>, report: &mut Report) {
    let sh_changed = changed(repo_root, base, r"\.sh$");
    if sh_changed.is_empty() {
        report.note("no changed shell scripts; bash -n skipped");
        return;
    }
    let mut syntax_bad = false;
    for file in &sh_changed {
        let path = repo_root.join(file);
        if !path.is_file() {
            continue;
        }
        let ok = bash()
            .arg("-n")
            .arg(path.to_string_lossy().replace('\\', "/"))
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false);
        if !ok {
            report.bad(&format!("bash -n: {file}"));
            syntax_bad = true;
        }
    }
    if !syntax_bad {
        report.ok("bash -n on changed scripts");
    }
}
