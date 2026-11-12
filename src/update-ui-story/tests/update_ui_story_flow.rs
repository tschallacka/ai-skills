// MODE: DEV
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};

fn plan(tag: &str) -> PathBuf {
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let mut dir = env::temp_dir();
    dir.push(format!(
        "update-ui-story-flow-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).unwrap();
    fs::write(
        dir.join("work-unit-inventory.md"),
        "# Work-unit inventory\n\n## Definition-of-done coverage\n\n| Required outcome or proof | Work unit IDs | Notes |\n|---|---|---|\n| W09 named outcome | W01 | notes |\n\n## Work units\n\n| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |\n|---|---|---|---|---|---|---|---|---|\n| W01 | verification | N/A | N/A | N/A | one | -- | 01-x | 01-a |\n| W89 | verification | N/A | N/A | N/A | two | -- | 01-x | 02-b |\n",
    )
    .unwrap();
    fs::write(
        dir.join("ui-user-stories.md"),
        "# UI user stories: x\n\n| ID | Persona | Actions | Interaction | Expected | Status | Evidence | Related work units | Cache |\n|---|---|---|---|---|---|---|---|---|\n| US-08 | a user | click save | click | saved | 💤 untested | — | W69,W89 | `ui-story-runs/US-08.md` |\n",
    )
    .unwrap();
    dir
}

fn run(plan: &Path, units: &str) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_update-ui-story"))
        .args([plan.to_str().unwrap(), "US-08", "--work-units", units])
        .output()
        .expect("run update-ui-story")
}

#[test]
fn work_units_replaces_the_whole_link_list_gh195() {
    let plan = plan("replace");
    let output = run(&plan, "W01, W89");
    assert!(
        output.status.success(),
        "update-ui-story failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stories = fs::read_to_string(plan.join("ui-user-stories.md")).unwrap();
    assert!(
        stories.contains("| — | W01,W89 | `ui-story-runs/US-08.md` |"),
        "{stories}"
    );
    assert!(!stories.contains("W69"), "stale link survived: {stories}");
    assert!(
        stories.contains("| a user | click save |"),
        "other cells changed: {stories}"
    );
    let _ = fs::remove_dir_all(&plan);
}

#[test]
fn an_unknown_work_unit_is_refused_and_changes_nothing_gh195() {
    let plan = plan("unknown");
    let before = fs::read_to_string(plan.join("ui-user-stories.md")).unwrap();
    // W09 appears only as text in a coverage outcome, never as a unit row.
    for units in ["W01,W69", "W09"] {
        let output = run(&plan, units);
        assert_eq!(output.status.code(), Some(66), "{units}");
        assert!(
            String::from_utf8_lossy(&output.stderr).contains("Related work unit not found"),
            "{units}"
        );
    }
    let output = run(&plan, "W01,");
    assert_eq!(output.status.code(), Some(64));
    assert_eq!(
        fs::read_to_string(plan.join("ui-user-stories.md")).unwrap(),
        before
    );
    let _ = fs::remove_dir_all(&plan);
}
