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
use std::process::{Command, Output};

/// Runs `cargo <subcommand> <extra...> --manifest-path <manifest>`, keeping its
/// output. `None` means cargo could not be started at all.
fn cargo_on(subcommand: &str, extra: &[&str], manifest: &Path) -> Option<Output> {
    Command::new("cargo")
        .arg(subcommand)
        .args(extra)
        .arg("--manifest-path")
        .arg(manifest)
        .output()
        .ok()
}

fn passed(out: &Option<Output>) -> bool {
    out.as_ref().is_some_and(|o| o.status.success())
}

/// Prints what a failed cargo run said. The gate used to keep only the exit
/// status, so a failure that depends on the machine -- another test run
/// holding the same port, a loaded runner -- left a bare `FAIL` line and
/// nothing to diagnose it from, and vanished when the gate was re-run.
///
/// For `cargo test` the useful part starts at the `failures:` section, which
/// carries each failed test's captured output; a build error has no such
/// section, so the tail of the output is shown instead.
fn print_evidence(out: &Option<Output>, keep: usize) {
    let Some(out) = out else {
        eprintln!("    (cargo could not be started)");
        return;
    };
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    for line in evidence_lines(&combined, keep) {
        eprintln!("    {line}");
    }
}

/// The part of a failed cargo run worth showing: from the first `failures:`
/// line when there is one, otherwise the last `keep` lines; at most `keep`.
fn evidence_lines(combined: &str, keep: usize) -> Vec<&str> {
    let lines: Vec<&str> = combined.lines().collect();
    let start = lines
        .iter()
        .position(|line| *line == "failures:")
        .unwrap_or_else(|| lines.len().saturating_sub(keep));
    lines.into_iter().skip(start).take(keep).collect()
}

fn gate_rust_crates_fmt_and_test(repo_root: &Path, crates: &[String], report: &mut Report) {
    for crate_name in crates {
        let manifest = repo_root.join("src").join(crate_name).join("Cargo.toml");
        if !manifest.is_file() {
            continue;
        }
        let fmt = cargo_on("fmt", &["--check"], &manifest);
        if passed(&fmt) {
            report.ok(&format!("cargo fmt --check: {crate_name}"));
        } else {
            report.bad(&format!(
                "cargo fmt --check: {crate_name} (CI runs fmt before the build)"
            ));
            print_evidence(&fmt, 40);
        }

        let test = cargo_on("test", &[], &manifest);
        if passed(&test) {
            report.ok(&format!("cargo test: {crate_name}"));
        } else {
            report.bad(&format!("cargo test: {crate_name}"));
            print_evidence(&test, 80);
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

#[cfg(test)]
mod tests {
    use super::evidence_lines;

    #[test]
    fn a_failed_test_run_is_shown_from_its_failures_section() {
        let text = "running 3 tests\ntest a ... ok\ntest b ... FAILED\n\nfailures:\n\n\
                    ---- b stdout ----\nthread 'b' panicked at x.rs:1:1\n\nfailures:\n    b\n\n\
                    test result: FAILED. 2 passed; 1 failed\nerror: test failed";
        let shown = evidence_lines(text, 80);
        assert_eq!(shown[0], "failures:");
        assert!(shown.iter().any(|l| l.contains("panicked at x.rs:1:1")));
        assert!(!shown.iter().any(|l| l.contains("test a ... ok")));
    }

    #[test]
    fn a_run_with_no_failures_section_shows_its_tail() {
        let text = (1..=100)
            .map(|n| format!("line {n}"))
            .collect::<Vec<_>>()
            .join("\n");
        let shown = evidence_lines(&text, 5);
        assert_eq!(
            shown,
            ["line 96", "line 97", "line 98", "line 99", "line 100"]
        );
    }

    #[test]
    fn the_excerpt_never_exceeds_the_cap() {
        let text = format!("failures:\n{}", "x\n".repeat(500));
        assert_eq!(evidence_lines(&text, 80).len(), 80);
    }
}
