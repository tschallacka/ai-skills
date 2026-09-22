// MODE: DEV
// PACKAGE: PROD

//! Gate 5: register soundness via reg_findings, the shipped implementation
//! (planning/scripts/register-lib.sh), needing rjq on PATH -- quietly
//! skipped without it (CI runs test-register-schemas regardless). Shells to
//! bash to source the real function rather than reimplementing it.

use crate::platform::{bash, which};
use crate::report::Report;
use std::path::Path;

fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

pub fn gate_register_soundness(repo_root: &Path, report: &mut Report) {
    if !which("rjq") {
        report.note("rjq not on PATH; register soundness skipped (CI runs test-register-schemas)");
        return;
    }
    let lib = repo_root.join("planning/scripts/register-lib.sh");
    for (register, kind) in [("TODO.json", "todo"), ("BUGS.json", "bug")] {
        let path = repo_root.join(register);
        if !path.is_file() {
            continue;
        }
        let script = format!(
            "set -euo pipefail; source {}; reg_findings {} {} 2>&1",
            // Forward slashes: bash does not reliably read a backslash path.
            shell_quote(&lib.to_string_lossy().replace('\\', "/")),
            shell_quote(kind),
            shell_quote(register),
        );
        let output = bash()
            .arg("-c")
            .arg(&script)
            .current_dir(repo_root)
            .output();
        match output {
            Ok(out) => {
                let findings = String::from_utf8_lossy(&out.stdout).trim().to_string();
                if !out.status.success() {
                    report.bad(&format!(
                        "{register}: the soundness check could not run: {findings}"
                    ));
                } else if !findings.is_empty() {
                    report.bad(&format!("{register}: {findings}"));
                } else {
                    report.ok(&format!("{register} is sound (reg_findings)"));
                }
            }
            Err(error) => {
                report.bad(&format!(
                    "{register}: the soundness check could not run: {error}"
                ));
            }
        }
    }
}
