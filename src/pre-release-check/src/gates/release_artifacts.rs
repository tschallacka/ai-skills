// MODE: DEV
// PACKAGE: PROD

//! RELEASE.md steps 5-6: the tarball and the npm package, each already
//! verified byte-for-byte by its own dedicated test -- invoked, never
//! reimplemented. Each builds a real tarball or runs a real `npm pack`, so
//! this gate is the slow one; running both here, in sequence, rather than
//! leaving them to the caller's own shell is also what avoids the test
//! interference a concurrent `npm pack` and `run-tests.sh` hit this release
//! (two processes both exercising tests/test-skill-files-manifest.sh at
//! once, one seeing the other's own scratch probe value).

use crate::platform::script_command;
use crate::report::Report;
use std::path::Path;

fn run_test(repo_root: &Path, script: &str, ok_label: &str, bad_label: &str, report: &mut Report) {
    let status = script_command(&repo_root.join(script))
        .current_dir(repo_root)
        .status();
    match status {
        Ok(s) if s.success() => report.ok(ok_label),
        _ => report.bad(bad_label),
    }
}

pub fn gate_release_artifacts(repo_root: &Path, report: &mut Report) {
    run_test(
        repo_root,
        "tests/test-release-package.sh",
        "the release tarball holds exactly its own declared file set",
        "the release tarball disagrees with its own declared file set -- see tests/test-release-package.sh",
        report,
    );
    run_test(
        repo_root,
        "planning/tests/test-npm-package.sh",
        "npm package matches the owned baseline (planning/tests/fixtures/overview/npm-package-baseline.tsv)",
        "npm package differs from its baseline -- regenerate planning/tests/fixtures/overview/npm-package-baseline.tsv",
        report,
    );
}
