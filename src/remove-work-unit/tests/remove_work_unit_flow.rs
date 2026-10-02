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
        "remove-work-unit-flow-{tag}-{}-{}",
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
        "# Goal: X\n\n## Owned work units\n\n§ 9.1\n`W01` — test change\n\n## Testing requirement\n\n| Test required | Rationale |\n|---|---|\n| no | n/a |\n\n## Goal-size exception\n",
    )
    .unwrap();
    fs::write(plan.join("01-x/steps/01-step-x.md"), "# Step: 01-step-x\n").unwrap();
    fs::write(
        plan.join("work-unit-inventory.md"),
        "# Work-unit inventory\n\n## Work units\n\n| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |\n|---|---|---|---|---|---|---|---|---|\n| W01 | discovery | N/A | N/A | N/A | test change | — | 01-x | 01-step-x |\n\n## Decomposition review\n\n- [ ] placeholder\n",
    )
    .unwrap();
    // Present so rebuild_plan_progress's own file_name() call is actually reached.
    fs::write(plan.join("progress.md"), "# Progress\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_remove-work-unit"))
        .args([".", "W01"])
        .current_dir(&plan)
        .output()
        .expect("run remove-work-unit");
    assert!(
        output.status.success(),
        "remove-work-unit failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !String::from_utf8_lossy(&output.stderr).contains("panicked"),
        "remove-work-unit panicked: {}",
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
