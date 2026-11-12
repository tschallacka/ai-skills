// MODE: DEV
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
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

/// A plan whose coverage outcome "Works end to end" starts with W, and whose
/// UI stories link the unit being removed -- the shape GH #194 reported.
fn cascade_plan(tag: &str, us02_units: &str) -> PathBuf {
    let plan = unique_dir(tag);
    fs::create_dir_all(plan.join("01-x/steps")).unwrap();
    fs::write(
        plan.join("01-x/goal.md"),
        "# Goal: X\n\n## Owned work units\n\n§ 9.1\n`W01` — one\n\n## Testing requirement\n\n| Test required | Rationale |\n|---|---|\n| no | n/a |\n\n## Goal-size exception\n",
    )
    .unwrap();
    for step in ["01-a", "02-b", "03-c"] {
        fs::write(
            plan.join(format!("01-x/steps/{step}.md")),
            format!("# Step: {step}\n"),
        )
        .unwrap();
    }
    fs::write(
        plan.join("work-unit-inventory.md"),
        "# Work-unit inventory\n\n## Definition-of-done coverage\n\n| Required outcome or proof | Work unit IDs | Notes |\n|---|---|---|\n| Outcome one | W01,W02 | notes |\n| Works end to end | W02,W03 | notes |\n\n## Work units\n\n| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |\n|---|---|---|---|---|---|---|---|---|\n| W01 | verification | N/A | N/A | N/A | one | -- | 01-x | 01-a |\n| W02 | verification | N/A | N/A | N/A | two | -- | 01-x | 02-b |\n| W03 | verification | N/A | N/A | N/A | three | -- | 01-x | 03-c |\n",
    )
    .unwrap();
    fs::write(
        plan.join("ui-user-stories.md"),
        format!("# UI user stories: x\n\n| ID | Persona | Actions | Interaction | Expected | Status | Evidence | Related work units | Cache |\n|---|---|---|---|---|---|---|---|---|\n| US-01 | a user | click save | click | saved | 💤 untested | — | W01,W02,W03 | `ui-story-runs/US-01.md` |\n| US-02 | a user | click save | click | saved | 💤 untested | — | {us02_units} | `ui-story-runs/US-02.md` |\n"),
    )
    .unwrap();
    plan
}

fn remove(plan: &Path, extra: &[&str]) -> std::process::Output {
    let mut args = vec![plan.to_str().unwrap(), "W02"];
    args.extend_from_slice(extra);
    Command::new(env!("CARGO_BIN_EXE_remove-work-unit"))
        .args(&args)
        .output()
        .expect("run remove-work-unit")
}

#[test]
fn story_links_need_confirm_cascade_and_are_named_gh194() {
    let plan = cascade_plan("story-refuse", "W01,W03");
    let before = fs::read_to_string(plan.join("work-unit-inventory.md")).unwrap();
    let output = remove(&plan, &[]);
    assert_eq!(output.status.code(), Some(73));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("US-01"),
        "refusal does not name the story: {stderr}"
    );
    assert_eq!(
        fs::read_to_string(plan.join("work-unit-inventory.md")).unwrap(),
        before,
        "a refusal must change nothing"
    );
    let _ = fs::remove_dir_all(&plan);
}

#[test]
fn confirm_cascade_prunes_coverage_and_story_links_gh194() {
    let plan = cascade_plan("story-prune", "W01,W03");
    let output = remove(&plan, &["--confirm-cascade"]);
    assert!(
        output.status.success(),
        "remove-work-unit failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let inventory = fs::read_to_string(plan.join("work-unit-inventory.md")).unwrap();
    assert!(
        inventory.contains("| Outcome one | W01 | notes |"),
        "{inventory}"
    );
    assert!(
        inventory.contains("| Works end to end | W03 | notes |"),
        "coverage row whose outcome starts with W kept W02: {inventory}"
    );
    assert!(!inventory.contains("| W02 |"), "{inventory}");
    let stories = fs::read_to_string(plan.join("ui-user-stories.md")).unwrap();
    assert!(
        stories.contains("| W01,W03 | `ui-story-runs/US-01.md` |"),
        "{stories}"
    );
    assert!(
        stories.contains("| W01,W03 | `ui-story-runs/US-02.md` |"),
        "US-02 untouched: {stories}"
    );
    assert!(!plan.join("01-x/steps/02-b.md").exists());
    let _ = fs::remove_dir_all(&plan);
}

#[test]
fn a_story_left_with_no_links_refuses_even_with_confirm_cascade_gh194() {
    let plan = cascade_plan("story-emptied", "W02");
    let output = remove(&plan, &["--confirm-cascade"]);
    assert_eq!(output.status.code(), Some(73));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("US-02"), "{stderr}");
    assert!(stderr.contains("update-ui-story.sh"), "{stderr}");
    assert!(
        plan.join("01-x/steps/02-b.md").exists(),
        "a refusal must change nothing"
    );
    let _ = fs::remove_dir_all(&plan);
}
