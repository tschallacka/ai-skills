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
        "create-plan-progress-flow-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_dot_plan_directory_does_not_panic_b338() {
    let plan = unique_dir("dot-arg");
    fs::create_dir_all(plan.join("01-a-goal")).unwrap();
    fs::write(plan.join("01-a-goal/goal.md"), "# Goal: A\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_create-plan-progress"))
        .arg(".")
        .current_dir(&plan)
        .output()
        .expect("run create-plan-progress");
    assert!(
        output.status.success(),
        "create-plan-progress failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !String::from_utf8_lossy(&output.stderr).contains("panicked"),
        "create-plan-progress panicked: {}",
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
