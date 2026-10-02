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
        "add-work-unit-flow-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_dot_plan_directory_does_not_panic_b338() {
    let plan = unique_dir("dot-arg");
    fs::create_dir_all(plan.join("01-x/steps")).unwrap();
    fs::write(
        plan.join("01-x/goal.md"),
        "# Goal: X\n\n## Owned work units\n\n§ 9.1\n<add work units with add-work-unit.sh>\n\n## Testing requirement\n\n| Test required | Rationale |\n|---|---|\n| no | n/a |\n",
    )
    .unwrap();
    fs::write(
        plan.join("work-unit-inventory.md"),
        "# Work-unit inventory\n\n## Work units\n\n| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |\n|---|---|---|---|---|---|---|---|---|\n\n## Decomposition review\n\n- [ ] placeholder\n",
    )
    .unwrap();
    // Present so make_plan_progress's own file_name() call is actually reached.
    fs::write(plan.join("progress.md"), "# Progress\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_add-work-unit"))
        .args([
            ".",
            "--id",
            "W01",
            "--type",
            "discovery",
            "--file",
            "N/A",
            "--scope",
            "N/A",
            "--subscope",
            "N/A",
            "--change",
            "test change",
            "--depends-on",
            "--",
            "--goal",
            "01-x",
            "--step",
            "01-step-x",
        ])
        .current_dir(&plan)
        .output()
        .expect("run add-work-unit");
    assert!(
        output.status.success(),
        "add-work-unit failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !String::from_utf8_lossy(&output.stderr).contains("panicked"),
        "add-work-unit panicked: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let progress = fs::read_to_string(plan.join("progress.md")).unwrap();
    let expected_name = plan
        .canonicalize()
        .unwrap()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    assert!(progress.contains(&format!("# Progress: {expected_name}")));

    let _ = fs::remove_dir_all(&plan);
}

fn scaffold_plan(tag: &str) -> PathBuf {
    let plan = unique_dir(tag);
    fs::create_dir_all(plan.join("01-x/steps")).unwrap();
    fs::write(
        plan.join("01-x/goal.md"),
        "# Goal: X\n\n## Owned work units\n\n§ 9.1\n<add work units with add-work-unit.sh>\n\n## Testing requirement\n\n| Test required | Rationale |\n|---|---|\n| no | n/a |\n",
    )
    .unwrap();
    fs::write(
        plan.join("work-unit-inventory.md"),
        "# Work-unit inventory\n\n## Work units\n\n| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |\n|---|---|---|---|---|---|---|---|---|\n\n## Decomposition review\n\n- [ ] placeholder\n",
    )
    .unwrap();
    fs::write(plan.join("progress.md"), "# Progress\n").unwrap();
    plan
}

fn add_work_unit(plan: &PathBuf, args: &[&str]) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_add-work-unit"))
        .arg(".")
        .args(args)
        .current_dir(plan)
        .output()
        .expect("run add-work-unit")
}

/// A relocation unit's File may name a directory (trailing slash), and its
/// Scope carries the destination rather than a symbol -- the two column
/// reinterpretations this kind is built on.
#[test]
fn a_relocation_unit_accepts_a_directory_file_and_a_destination_scope() {
    let plan = scaffold_plan("relocation-basic");
    let output = add_work_unit(
        &plan,
        &[
            "--id",
            "W01",
            "--type",
            "relocation",
            "--file",
            "old/data/",
            "--scope",
            "new/data/",
            "--subscope",
            "N/A",
            "--change",
            "Move the legacy data directory to its new home, unchanged",
            "--depends-on",
            "--",
            "--goal",
            "01-x",
            "--step",
            "01-step-x",
        ],
    );
    assert!(
        output.status.success(),
        "relocation unit was refused: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let inventory = fs::read_to_string(plan.join("work-unit-inventory.md")).unwrap();
    assert!(
        inventory.contains("| W01 | relocation | `old/data/` | `new/data/` |"),
        "row missing or malformed: {inventory}"
    );
    let step = fs::read_to_string(plan.join("01-x/steps/01-step-x.md")).unwrap();
    assert!(step.contains("- File: `old/data/`"), "{step}");
    assert!(
        step.contains("- Primary symbol or file scope: `new/data/`"),
        "{step}"
    );
    let _ = fs::remove_dir_all(&plan);
}

/// A relocation unit's source may just as well be one file -- a database
/// dump, say -- not only a directory; only the directory-shaped case needed
/// the exemption from the usual "one concrete file" rule.
#[test]
fn a_relocation_unit_accepts_a_single_file_source_too() {
    let plan = scaffold_plan("relocation-single-file");
    let output = add_work_unit(
        &plan,
        &[
            "--id",
            "W01",
            "--type",
            "relocation",
            "--file",
            "old/app.db",
            "--scope",
            "new/app.db",
            "--subscope",
            "N/A",
            "--change",
            "Move the database file to its new location, unchanged",
            "--depends-on",
            "--",
            "--goal",
            "01-x",
            "--step",
            "01-step-x",
        ],
    );
    assert!(
        output.status.success(),
        "relocation unit was refused: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let _ = fs::remove_dir_all(&plan);
}

/// The one column-shape rule relocation cannot waive: it still needs a
/// destination, so an N/A scope is refused the same way an N/A file is for
/// every other non-discovery, non-verification kind.
#[test]
fn a_relocation_unit_without_a_destination_scope_is_refused() {
    let plan = scaffold_plan("relocation-no-dest");
    let output = add_work_unit(
        &plan,
        &[
            "--id",
            "W01",
            "--type",
            "relocation",
            "--file",
            "old/data/",
            "--scope",
            "N/A",
            "--subscope",
            "N/A",
            "--change",
            "Move the data directory",
            "--depends-on",
            "--",
            "--goal",
            "01-x",
            "--step",
            "01-step-x",
        ],
    );
    assert!(!output.status.success(), "should have been refused");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("destination"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let _ = fs::remove_dir_all(&plan);
}

/// A non-relocation kind still cannot name a directory -- the exemption is
/// specific to relocation, not a general loosening of the glob/directory
/// rule.
#[test]
fn a_non_relocation_kind_still_refuses_a_directory_file() {
    let plan = scaffold_plan("relocation-not-other-kinds");
    let output = add_work_unit(
        &plan,
        &[
            "--id",
            "W01",
            "--type",
            "source",
            "--file",
            "src/lib/",
            "--scope",
            "some_fn",
            "--subscope",
            "N/A",
            "--change",
            "not a relocation",
            "--depends-on",
            "--",
            "--goal",
            "01-x",
            "--step",
            "01-step-x",
        ],
    );
    assert!(!output.status.success(), "should have been refused");
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("glob or directory"),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let _ = fs::remove_dir_all(&plan);
}

/// With --repo-root, a relocation unit's source is checked to actually exist
/// -- as a directory, which the ordinary single-file existence check would
/// reject outright.
#[test]
fn a_relocation_unit_with_repo_root_checks_the_source_directory_exists() {
    let plan = scaffold_plan("relocation-repo-root");
    let repo = unique_dir("relocation-repo-root-src");
    fs::create_dir_all(repo.join("old/data")).unwrap();
    fs::write(repo.join("old/data/file.txt"), "content").unwrap();

    let missing = add_work_unit(
        &plan,
        &[
            "--repo-root",
            repo.to_str().unwrap(),
            "--id",
            "W01",
            "--type",
            "relocation",
            "--file",
            "old/missing/",
            "--scope",
            "new/missing/",
            "--subscope",
            "N/A",
            "--change",
            "Move a directory that is not actually there",
            "--depends-on",
            "--",
            "--goal",
            "01-x",
            "--step",
            "01-step-x",
        ],
    );
    assert!(!missing.status.success(), "should have been refused");
    assert!(
        String::from_utf8_lossy(&missing.stderr).contains("does not exist"),
        "{}",
        String::from_utf8_lossy(&missing.stderr)
    );

    let present = add_work_unit(
        &plan,
        &[
            "--repo-root",
            repo.to_str().unwrap(),
            "--id",
            "W01",
            "--type",
            "relocation",
            "--file",
            "old/data/",
            "--scope",
            "new/data/",
            "--subscope",
            "N/A",
            "--change",
            "Move the real directory",
            "--depends-on",
            "--",
            "--goal",
            "01-x",
            "--step",
            "01-step-x",
        ],
    );
    assert!(
        present.status.success(),
        "a real source directory should have been accepted: {}",
        String::from_utf8_lossy(&present.stderr)
    );

    let _ = fs::remove_dir_all(&plan);
    let _ = fs::remove_dir_all(&repo);
}
