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
        "plan-content-summary-flow-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_dot_plan_directory_does_not_panic_b338() {
    let plan = unique_dir("dot-arg");
    fs::write(
        plan.join("work-unit-inventory.md"),
        "# Work-unit inventory\n\n## Work units\n\n| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |\n|---|---|---|---|---|---|---|---|---|\n",
    )
    .unwrap();
    for format in ["markdown", "text", "json"] {
        let output = Command::new(env!("CARGO_BIN_EXE_plan-content"))
            .args(["summary", ".", format])
            .current_dir(&plan)
            .output()
            .expect("run plan-content summary");
        assert!(
            output.status.success(),
            "plan-content summary {format} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            !String::from_utf8_lossy(&output.stderr).contains("panicked"),
            "plan-content summary {format} panicked: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let _ = fs::remove_dir_all(&plan);
}

// B336: a step whose own target script name ends in "-testing" (e.g. wiring
// create-step-testing.sh) must still be found under `--in steps`, not
// silently treated as a testing companion just because its id ends in the
// same literal substring.
#[test]
fn find_scope_steps_keeps_a_step_whose_own_name_ends_in_testing_b336() {
    let plan = unique_dir("testing-suffix-step");
    fs::create_dir_all(plan.join("g1/steps")).unwrap();
    fs::write(
        plan.join("work-unit-inventory.md"),
        "# Work-unit inventory\n\n## Work units\n\n| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |\n|---|---|---|---|---|---|---|---|---|\n",
    )
    .unwrap();
    fs::write(plan.join("g1/goal.md"), "# Goal: g1\n").unwrap();
    // No "wire-create-step.md" sibling exists, so this is a real step, not a
    // companion.
    fs::write(
        plan.join("g1/steps/wire-create-step-testing.md"),
        "# Step: wire-create-step-testing\n\n## Objective\n\nUniqueNeedleB336\n",
    )
    .unwrap();

    let found_in_steps = Command::new(env!("CARGO_BIN_EXE_plan-content"))
        .args(["find", ".", "UniqueNeedleB336", "--in", "steps"])
        .current_dir(&plan)
        .output()
        .expect("run plan-content find --in steps");
    assert!(
        found_in_steps.status.success(),
        "expected the step to be found under --in steps: stdout={} stderr={}",
        String::from_utf8_lossy(&found_in_steps.stdout),
        String::from_utf8_lossy(&found_in_steps.stderr)
    );
    assert!(
        String::from_utf8_lossy(&found_in_steps.stdout).contains("wire-create-step-testing"),
        "{}",
        String::from_utf8_lossy(&found_in_steps.stdout)
    );

    let found_in_testing = Command::new(env!("CARGO_BIN_EXE_plan-content"))
        .args(["find", ".", "UniqueNeedleB336", "--in", "testing"])
        .current_dir(&plan)
        .output()
        .expect("run plan-content find --in testing");
    assert!(
        !found_in_testing.status.success(),
        "the real step must not be misclassified as a testing companion"
    );

    let _ = fs::remove_dir_all(&plan);
}
