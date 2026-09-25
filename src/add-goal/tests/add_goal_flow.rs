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
        "add-goal-flow-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_dot_plan_directory_does_not_panic_b338() {
    let plan = unique_dir("dot-arg");
    let output = Command::new(env!("CARGO_BIN_EXE_add-goal"))
        .args([".", "01-test-goal", "Title", "Outcome"])
        .current_dir(&plan)
        .output()
        .expect("run add-goal");
    assert!(
        output.status.success(),
        "add-goal failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !String::from_utf8_lossy(&output.stderr).contains("panicked"),
        "add-goal panicked: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(plan.join("01-test-goal/goal.md").is_file());
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
