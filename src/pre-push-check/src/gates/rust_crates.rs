// MODE: DEV
// PACKAGE: PROD

//! Gate 4: cargo fmt --check and cargo test for each crate under src/ the
//! change touches, then a workspace-wide cargo clippy. Workspace-wide, not
//! per-crate: a per-crate pass misses a library change breaking a consumer
//! the change itself did not touch.

use crate::change_set::changed;
use crate::platform::which;
use crate::report::Report;
use std::path::Path;
use std::process::Command;

fn gate_rust_crates_fmt_and_test(repo_root: &Path, crates: &[String], report: &mut Report) {
    for crate_name in crates {
        let manifest = repo_root.join("src").join(crate_name).join("Cargo.toml");
        if !manifest.is_file() {
            continue;
        }
        let fmt_ok = Command::new("cargo")
            .args(["fmt", "--check", "--manifest-path"])
            .arg(&manifest)
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false);
        if fmt_ok {
            report.ok(&format!("cargo fmt --check: {crate_name}"));
        } else {
            report.bad(&format!(
                "cargo fmt --check: {crate_name} (CI runs fmt before the build)"
            ));
        }

        let test_ok = Command::new("cargo")
            .args(["test", "--manifest-path"])
            .arg(&manifest)
            .output()
            .map(|out| out.status.success())
            .unwrap_or(false);
        if test_ok {
            report.ok(&format!("cargo test: {crate_name}"));
        } else {
            report.bad(&format!("cargo test: {crate_name}"));
        }
    }
}

fn gate_rust_crates_clippy(repo_root: &Path, report: &mut Report) {
    let output = Command::new("cargo")
        .args([
            "clippy",
            "--workspace",
            "--all-targets",
            "--",
            "-D",
            "warnings",
        ])
        .current_dir(repo_root)
        .output();
    let ok = output
        .as_ref()
        .map(|out| out.status.success())
        .unwrap_or(false);
    if ok {
        report.ok("cargo clippy --workspace -D warnings");
    } else {
        report.bad("cargo clippy --workspace -D warnings (CI gates on this too)");
        if let Ok(out) = output {
            let combined = format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            );
            for line in combined.lines().take(40) {
                eprintln!("{line}");
            }
        }
    }
}

pub fn gate_rust_crates(repo_root: &Path, base: Option<&str>, report: &mut Report) {
    let files = changed(repo_root, base, r"^src/[^/]+/");
    let mut crates: Vec<String> = files
        .iter()
        .filter_map(|f| {
            let rest = f.strip_prefix("src/")?;
            let name = rest.split('/').next()?;
            (!name.is_empty()).then(|| name.to_string())
        })
        .collect();
    crates.sort();
    crates.dedup();

    if crates.is_empty() {
        report.note("no crates under src/ changed; rust gates skipped");
        return;
    }
    if !which("cargo") {
        report.note(
            "src/ changed but cargo is not on PATH (nix develop); CI still runs fmt and test",
        );
        return;
    }
    gate_rust_crates_fmt_and_test(repo_root, &crates, report);
    gate_rust_crates_clippy(repo_root, report);
}
