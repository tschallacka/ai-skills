// MODE: DEV
// PACKAGE: PROD

//! Gate 3: shellcheck on the scripts that differ from the base, with -x so
//! `source=` resolves from disk. CI lints the whole live set; only the
//! change set is linted here, which is what makes this gate proportional to
//! the change rather than to the whole tree.

use crate::change_set::changed;
use crate::platform::{script_command, which};
use crate::report::Report;
use std::path::Path;
use std::process::Command;

fn print_first_lines(text: &str, limit: usize) {
    for line in text.lines().take(limit) {
        eprintln!("{line}");
    }
}

/// Builds the shared bundled libraries first (best-effort: a failure here
/// is not fatal), then collects the changed, still-existing shell scripts
/// and runs shellcheck against them. Returns that list so gate_static_scans
/// can reuse it without recomputing.
pub fn gate_shellcheck(
    repo_root: &Path,
    base: Option<&str>,
    base_label: Option<&str>,
    report: &mut Report,
) -> Vec<String> {
    let build_libs = repo_root.join("planning/scripts/build-plan-libs.sh");
    if build_libs.is_file() {
        let _ = script_command(&build_libs).current_dir(repo_root).output();
    }

    let changed_sh: Vec<String> = changed(repo_root, base, r"\.sh$")
        .into_iter()
        .filter(|f| repo_root.join(f).is_file())
        .collect();

    let base_desc = base_label.unwrap_or("the base");
    if changed_sh.is_empty() {
        report.note(&format!(
            "no shell scripts differ from {base_desc}; shellcheck skipped (CI lints all)"
        ));
        return changed_sh;
    }
    if !which("shellcheck") {
        report.note("shellcheck not installed locally; CI still gates on it");
        return changed_sh;
    }

    let mut args: Vec<&str> = vec!["-x", "-s", "bash", "--severity=warning"];
    args.extend(changed_sh.iter().map(String::as_str));
    let output = Command::new("shellcheck")
        .args(&args)
        .current_dir(repo_root)
        .output();
    let ok = output
        .as_ref()
        .map(|out| out.status.success())
        .unwrap_or(false);
    if ok {
        report.ok(&format!(
            "shellcheck -x --severity=warning ({} changed vs {})",
            changed_sh.len(),
            base_label.unwrap_or("base")
        ));
    } else {
        report.bad("shellcheck findings at warning severity (CI gates on these)");
        if let Ok(out) = output {
            let combined = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            print_first_lines(&combined, 40);
        }
    }
    changed_sh
}
