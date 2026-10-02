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
        "create-work-unit-inventory-flow-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_dot_plan_directory_does_not_panic_b338() {
    let plan = unique_dir("dot-arg");
    let output = Command::new(env!("CARGO_BIN_EXE_create-work-unit-inventory"))
        .arg(".")
        .current_dir(&plan)
        .output()
        .expect("run create-work-unit-inventory");
    assert!(
        output.status.success(),
        "create-work-unit-inventory failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !String::from_utf8_lossy(&output.stderr).contains("panicked"),
        "create-work-unit-inventory panicked: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let inventory = fs::read_to_string(plan.join("work-unit-inventory.md")).unwrap();
    let expected_name = plan
        .canonicalize()
        .unwrap()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    assert!(inventory.contains(&format!("# Work-unit inventory: {expected_name}")));

    let _ = fs::remove_dir_all(&plan);
}
