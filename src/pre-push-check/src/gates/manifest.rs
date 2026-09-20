// MODE: DEV
// PACKAGE: PROD

//! Gate 6: every tracked skill file is declared in skill_files(). The
//! baseline gate checks sizes of paths it already knows; this catches a file
//! added to a skill directory and never declared, which npm's own file
//! selection would otherwise only catch in CI.

use crate::platform::script_command;
use crate::report::Report;
use std::path::Path;

pub fn gate_skill_manifest(repo_root: &Path, report: &mut Report) {
    let manifest_test = repo_root.join("tests/test-skill-files-manifest.sh");
    if !manifest_test.is_file() {
        report.note(&format!(
            "no {} to check skill declarations with",
            manifest_test.display()
        ));
        return;
    }
    let output = script_command(&manifest_test)
        .arg("--declarations-only")
        .current_dir(repo_root)
        .output();
    match output {
        Ok(out) if out.status.success() => {
            report.ok("every tracked skill file is declared in skill_files()");
        }
        Ok(out) => {
            report.bad("a skill file is tracked but not declared in skill_files()");
            let combined = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            for line in combined.lines() {
                if let Some(rest) = line.trim_start().strip_prefix("FAIL: ") {
                    println!("  {rest}");
                }
            }
            report.note(
                "add it to the right arm in installer/src/50-manifest.sh, then installer/build.sh",
            );
        }
        Err(_) => {
            report.bad("a skill file is tracked but not declared in skill_files()");
        }
    }
}
