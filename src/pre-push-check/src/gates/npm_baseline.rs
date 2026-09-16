// MODE: DEV
// PACKAGE: PROD

//! Gate 5b: npm-package-baseline.tsv pins the byte size of every packaged
//! file; a packaged text file's bytes are the working tree's bytes, checked
//! directly here in milliseconds instead of running the minutes-long
//! `npm pack` the full test-npm-package.sh needs. Does not cover npm's file
//! *selection* -- only the full test remains authoritative for the file set.
//! An absent file is not drift: some baseline rows name generated,
//! untracked artifacts a fresh checkout simply does not have.

use crate::report::Report;
use std::fs;
use std::path::Path;

pub enum BaselineOutcome {
    Drift(Vec<String>),
    Matches { total: u32, unchecked: u32 },
}

/// Pure over an already-read baseline file's content, so it can be unit
/// tested against a synthetic TSV without touching a real npm package tree.
pub fn evaluate_baseline(repo_root: &Path, content: &str) -> BaselineOutcome {
    let mut drift = Vec::new();
    let mut unchecked: u32 = 0;
    let mut total: u32 = 0;
    for line in content.lines().skip(1) {
        let mut parts = line.splitn(2, '\t');
        let Some(pkgpath) = parts.next() else {
            continue;
        };
        let size = parts.next().unwrap_or("").trim();
        if pkgpath.is_empty() {
            continue;
        }
        total += 1;
        let repopath = pkgpath.strip_prefix("package/").unwrap_or(pkgpath);
        let full = repo_root.join(repopath);
        let Ok(metadata) = fs::metadata(&full) else {
            unchecked += 1;
            continue;
        };
        let actual = metadata.len().to_string();
        if actual != size {
            drift.push(format!(
                "{repopath} is {actual} bytes, baseline says {size}"
            ));
        }
    }
    if drift.is_empty() {
        BaselineOutcome::Matches { total, unchecked }
    } else {
        BaselineOutcome::Drift(drift)
    }
}

pub fn gate_npm_baseline(repo_root: &Path, report: &mut Report) {
    let relative = "planning/tests/fixtures/overview/npm-package-baseline.tsv";
    let baseline = repo_root.join(relative);
    let Ok(content) = fs::read_to_string(&baseline) else {
        report.note(&format!("no npm package baseline at {relative}"));
        return;
    };

    match evaluate_baseline(repo_root, &content) {
        BaselineOutcome::Drift(drift) => {
            report.bad(&format!("npm package baseline drift: {}", drift.join("; ")));
            report.note("refresh the rows, then confirm with planning/tests/test-npm-package.sh");
        }
        BaselineOutcome::Matches { total, unchecked } if unchecked > 0 => {
            report.ok(&format!(
                "npm package baseline matches the tree ({} of {} files; {} generated and not built here)",
                total - unchecked,
                total,
                unchecked
            ));
        }
        BaselineOutcome::Matches { total, .. } => {
            report.ok(&format!(
                "npm package baseline matches the tree ({total} files)"
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn scratch(tag: &str) -> std::path::PathBuf {
        let mut dir = std::env::temp_dir();
        dir.push(format!(
            "pre-push-check-npm-baseline-{tag}-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_matching_file_reports_no_drift() {
        let root = scratch("match");
        fs::write(root.join("a.txt"), "hello").unwrap();
        let tsv = "path\tsize\npackage/a.txt\t5\n";
        match evaluate_baseline(&root, tsv) {
            BaselineOutcome::Matches { total, unchecked } => {
                assert_eq!(total, 1);
                assert_eq!(unchecked, 0);
            }
            BaselineOutcome::Drift(_) => panic!("expected no drift"),
        }
    }

    #[test]
    fn a_wrong_size_is_reported_as_drift_naming_both_sizes() {
        let root = scratch("drift");
        fs::write(root.join("a.txt"), "hello").unwrap();
        let tsv = "path\tsize\npackage/a.txt\t999\n";
        match evaluate_baseline(&root, tsv) {
            BaselineOutcome::Drift(drift) => {
                assert_eq!(drift.len(), 1);
                assert!(drift[0].contains("a.txt is 5 bytes, baseline says 999"));
            }
            BaselineOutcome::Matches { .. } => panic!("expected drift"),
        }
    }

    #[test]
    fn a_missing_file_is_unchecked_rather_than_drift() {
        let root = scratch("missing");
        let tsv = "path\tsize\npackage/generated.md\t42\n";
        match evaluate_baseline(&root, tsv) {
            BaselineOutcome::Matches { total, unchecked } => {
                assert_eq!(total, 1);
                assert_eq!(unchecked, 1);
            }
            BaselineOutcome::Drift(_) => panic!("a missing file must not count as drift"),
        }
    }
}
