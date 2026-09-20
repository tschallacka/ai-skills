// MODE: DEV
// PACKAGE: PROD

//! Gate 3b: the two static shell gates CI fails on, over the same changed
//! set gate 3 already built -- the function-length cap (a ratchet: reports
//! only what this change is responsible for) and the portability-construct
//! scan (only sees the changed files, not the whole tree).

use crate::platform::script_command;
use crate::report::Report;
use std::path::Path;

fn print_first_lines(text: &str, limit: usize) {
    for line in text.lines().take(limit) {
        eprintln!("{line}");
    }
}

fn gate_static_scans_cap(
    repo_root: &Path,
    base: Option<&str>,
    changed_sh: &[String],
    report: &mut Report,
) {
    let cap_test = repo_root.join("planning/tests/test-function-length-ratchet.sh");
    if !cap_test.is_file() {
        report.note(&format!(
            "no {} to check the function cap with",
            cap_test.display()
        ));
        return;
    }
    let mut args: Vec<String> = vec!["--files".into(), "--base".into(), base.unwrap_or("").into()];
    args.extend(changed_sh.iter().cloned());
    let output = script_command(&cap_test)
        .args(&args)
        .current_dir(repo_root)
        .output();
    let ok = output
        .as_ref()
        .map(|out| out.status.success())
        .unwrap_or(false);
    if ok {
        report.ok("no function newly over the 40-line cap");
    } else {
        report.bad("this change puts a function over CODE-STYLE.md's 40-line cap");
        if let Ok(out) = output {
            let combined = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            print_first_lines(&combined, 20);
        }
    }
}

fn gate_static_scans_portability(repo_root: &Path, changed_sh: &[String], report: &mut Report) {
    let port_test = repo_root.join("planning/tests/test-portability-contract.sh");
    if !port_test.is_file() {
        report.note(&format!(
            "no {} to check portability constructs with",
            port_test.display()
        ));
        return;
    }
    let mut args: Vec<String> = vec!["--files".into()];
    args.extend(changed_sh.iter().cloned());
    let output = script_command(&port_test)
        .args(&args)
        .current_dir(repo_root)
        .output();
    let ok = output
        .as_ref()
        .map(|out| out.status.success())
        .unwrap_or(false);
    if ok {
        report.ok("no banned portability construct in the changed scripts");
    } else {
        report.bad("a changed script uses a construct PORTABILITY.md bans");
        if let Ok(out) = output {
            let combined = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            print_first_lines(&combined, 20);
        }
    }
}

pub fn gate_static_scans(
    repo_root: &Path,
    base: Option<&str>,
    base_label: Option<&str>,
    changed_sh: &[String],
    report: &mut Report,
) {
    if changed_sh.is_empty() {
        report.note(&format!(
            "no shell scripts differ from {}; cap and portability scans skipped",
            base_label.unwrap_or("the base")
        ));
        return;
    }
    gate_static_scans_cap(repo_root, base, changed_sh, report);
    gate_static_scans_portability(repo_root, changed_sh, report);
}
