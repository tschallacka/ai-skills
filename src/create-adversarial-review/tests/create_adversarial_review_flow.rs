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
        "create-adversarial-review-flow-{tag}-{}-{}",
        std::process::id(),
        COUNTER.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn the_review_scope_block_carries_all_six_bullets_in_order() {
    let plan = unique_dir("six-bullets");
    let output = Command::new(env!("CARGO_BIN_EXE_create-adversarial-review"))
        .arg(&plan)
        .output()
        .expect("run create-adversarial-review");
    assert!(
        output.status.success(),
        "create-adversarial-review failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let review = fs::read_to_string(plan.join("adversarial-review.md")).unwrap();
    let scope_start = review
        .find("## Review scope")
        .expect("no Review scope section");
    let findings_start = review.find("## Findings").expect("no Findings section");
    let scope = &review[scope_start..findings_start];

    let order = [
        "- Request:",
        "- Repository/context inspected:",
        "- Reviewer session:",
        "- Elapsed:",
        "- Cost signal:",
        "- Tokens:",
    ];
    let mut last_pos = 0;
    for bullet in order {
        let pos = scope
            .find(bullet)
            .unwrap_or_else(|| panic!("missing bullet {bullet:?} in:\n{scope}"));
        assert!(
            pos >= last_pos,
            "bullet {bullet:?} out of order in:\n{scope}"
        );
        last_pos = pos;
    }

    let _ = fs::remove_dir_all(&plan);
}

#[test]
fn a_dot_plan_directory_does_not_panic_b338() {
    let plan = unique_dir("dot-arg");
    let output = Command::new(env!("CARGO_BIN_EXE_create-adversarial-review"))
        .arg(".")
        .current_dir(&plan)
        .output()
        .expect("run create-adversarial-review");
    assert!(
        output.status.success(),
        "create-adversarial-review failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        !String::from_utf8_lossy(&output.stderr).contains("panicked"),
        "create-adversarial-review panicked: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let review = fs::read_to_string(plan.join("adversarial-review.md")).unwrap();
    let expected_name = plan
        .canonicalize()
        .unwrap()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    assert!(review.contains(&format!("# Adversarial review: {expected_name}")));

    let _ = fs::remove_dir_all(&plan);
}
