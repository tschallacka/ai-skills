// MODE: DEV
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

fn unique_dir(tag: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let mut dir = env::temp_dir();
    dir.push(format!(
        "resolve-finding-flow-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn review_with_finding(work_unit_cell: &str) -> String {
    format!(
        "# Adversarial review: fixture\n\n\
## Findings\n\n\
| ID | Missing or over-broad item | Required plan change | Status | Work unit |\n\
|---|---|---|---|---|\n\
| AR-01 | something | fix it | open | {work_unit_cell} |\n"
    )
}

/// B357 regression: a finding with a blank Work-unit cell (a real, sanctioned
/// shape -- e.g. a finding about the plan description itself) must still be
/// resolvable by Status alone, with no fix-key claim required or attempted.
#[test]
fn an_ungated_finding_with_a_blank_work_unit_cell_resolves_without_a_fix_key() {
    let plan = unique_dir("ungated-blank");
    fs::write(plan.join("adversarial-review.md"), review_with_finding("")).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_resolve-finding"))
        .arg(&plan)
        .arg("AR-01")
        .output()
        .expect("run resolve-finding");
    assert!(
        output.status.success(),
        "resolve-finding failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("ungated"),
        "expected an ungated confirmation, got: {stdout}"
    );

    let review = fs::read_to_string(plan.join("adversarial-review.md")).unwrap();
    let row = review
        .lines()
        .find(|line| line.starts_with("| AR-01"))
        .expect("AR-01 row missing");
    assert!(
        row.contains("resolved"),
        "expected the Status cell to flip to resolved, row was: {row}"
    );
    assert!(
        !plan.join("fixes.md").exists(),
        "an ungated finding must never write a fix claim"
    );
}

/// Same as above, but with an explicit N/A cell rather than an empty one --
/// both are real shapes this table uses for "no work unit".
#[test]
fn an_ungated_finding_with_an_n_a_work_unit_cell_resolves_without_a_fix_key() {
    let plan = unique_dir("ungated-na");
    fs::write(
        plan.join("adversarial-review.md"),
        review_with_finding("N/A"),
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_resolve-finding"))
        .arg(&plan)
        .arg("AR-01")
        .output()
        .expect("run resolve-finding");
    assert!(
        output.status.success(),
        "resolve-finding failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let review = fs::read_to_string(plan.join("adversarial-review.md")).unwrap();
    let row = review
        .lines()
        .find(|line| line.starts_with("| AR-01"))
        .expect("AR-01 row missing");
    assert!(row.contains("resolved"), "row was: {row}");
}

/// A gated finding (a real WNN cell) with no fix-keys.json yet still resolves
/// the Status cell, matching the pre-existing behavior this fix must not
/// regress -- it just cannot record a claim without keys to claim from.
#[test]
fn a_gated_finding_with_no_fix_keys_file_still_resolves_status() {
    let plan = unique_dir("gated-no-keys");
    fs::write(
        plan.join("adversarial-review.md"),
        review_with_finding("W01"),
    )
    .unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_resolve-finding"))
        .arg(&plan)
        .arg("AR-01")
        .output()
        .expect("run resolve-finding");
    assert!(
        output.status.success(),
        "resolve-finding failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let review = fs::read_to_string(plan.join("adversarial-review.md")).unwrap();
    let row = review
        .lines()
        .find(|line| line.starts_with("| AR-01"))
        .expect("AR-01 row missing");
    assert!(row.contains("resolved"), "row was: {row}");
}
