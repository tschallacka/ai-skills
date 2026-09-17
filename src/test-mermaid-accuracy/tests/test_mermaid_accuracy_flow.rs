// MODE: DEV
// PACKAGE: PROD

//! Drives the real built binary end to end. The real, live tracked documents
//! are this suite's own actual value (a synthetic-only test suite would not
//! prove the port works against the thing it exists to check), so the
//! primary assertion here runs against the real repository, not a fixture.

use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is src/test-mermaid-accuracy; the repo root is two
    // levels up.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn run_against(root: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_test-mermaid-accuracy"))
        .env("PLANNING_SKILL_ROOT", root)
        .output()
        .expect("binary runs")
}

#[test]
fn the_real_live_tracked_documents_pass_with_no_findings() {
    let output = run_against(&repo_root());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("test-mermaid-accuracy: PASS"),
        "stdout: {stdout}\nstderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    // The real, live corpus reality this port was verified against (see
    // goal.md's own AR-133/AR-134 citations): 54 scripts, 24 artifacts, 3
    // function names -- confirming the four tracked documents actually got
    // scanned, not merely that the process exited zero.
    assert!(stdout.contains("54 scripts and 24 artifacts named in diagrams: PASS"));
    assert!(stdout.contains("3 function names in the diagram documents: PASS"));
    assert!(output.status.success());
}

#[test]
fn a_malformed_synthetic_document_produces_the_exact_fail_message_and_line() {
    let scratch = std::env::temp_dir().join(format!(
        "test-mermaid-accuracy-malformed-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&scratch);
    let planning = scratch.join("planning");
    let benchmark_planning = scratch.join("benchmark").join("planning");
    let brainstorm = scratch.join("brainstorm");
    let review = scratch.join("post-implementation-review");
    for dir in [&planning, &benchmark_planning, &brainstorm, &review] {
        std::fs::create_dir_all(dir).unwrap();
    }
    std::fs::create_dir_all(scratch.join("planning/scripts")).unwrap();
    // A malformed block: an undefined node id (B never given a labeled form).
    let malformed = "# doc\n\n```mermaid\nflowchart TD\n    A-->B\n```\n";
    std::fs::write(planning.join("ARCHITECTURE.md"), malformed).unwrap();
    std::fs::write(
        benchmark_planning.join("ARCHITECTURE.md"),
        "```mermaid\nflowchart TD\n    A[x]-->B[y]\n```\n",
    )
    .unwrap();
    std::fs::write(
        brainstorm.join("SKILL.md"),
        "```mermaid\nflowchart TD\n    A[x]-->B[y]\n```\n",
    )
    .unwrap();
    std::fs::write(
        review.join("SKILL.md"),
        "```mermaid\nflowchart TD\n    A[x]-->B[y]\n```\n",
    )
    .unwrap();

    let output = run_against(&scratch);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("planning/ARCHITECTURE.md:5: node id never defined: A"),
        "{stderr}"
    );
    assert!(
        stderr.contains("planning/ARCHITECTURE.md:5: node id never defined: B"),
        "{stderr}"
    );
    assert!(!output.status.success());
    let _ = std::fs::remove_dir_all(&scratch);
}

#[test]
fn a_zero_fence_document_fails_by_name_distinct_from_the_blocks_cross_check() {
    let scratch = std::env::temp_dir().join(format!(
        "test-mermaid-accuracy-zero-fence-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&scratch);
    for dir in [
        scratch.join("planning"),
        scratch.join("benchmark/planning"),
        scratch.join("brainstorm"),
        scratch.join("post-implementation-review"),
        scratch.join("planning/scripts"),
    ] {
        std::fs::create_dir_all(dir).unwrap();
    }
    // No mermaid fence at all in this one.
    std::fs::write(scratch.join("planning/ARCHITECTURE.md"), "# no diagrams here\n").unwrap();
    std::fs::write(
        scratch.join("benchmark/planning/ARCHITECTURE.md"),
        "```mermaid\nflowchart TD\n    A[x]-->B[y]\n```\n",
    )
    .unwrap();
    std::fs::write(
        scratch.join("brainstorm/SKILL.md"),
        "```mermaid\nflowchart TD\n    A[x]-->B[y]\n```\n",
    )
    .unwrap();
    std::fs::write(
        scratch.join("post-implementation-review/SKILL.md"),
        "```mermaid\nflowchart TD\n    A[x]-->B[y]\n```\n",
    )
    .unwrap();

    let output = run_against(&scratch);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("planning/ARCHITECTURE.md has no mermaid diagram"),
        "{stderr}"
    );
    let _ = std::fs::remove_dir_all(&scratch);
}

#[test]
fn a_hyphen_led_artifact_resolves_via_the_broadened_match_on_the_real_live_document() {
    // AR-132: planning/ARCHITECTURE.md's own real, live content names the
    // literal token '-testing.md', which must resolve via the broadened,
    // suffix-anchored match rather than an exact basename lookup.
    let output = run_against(&repo_root());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        !stderr.contains("-testing.md"),
        "the real, live '-testing.md' token must not be flagged: {stderr}"
    );
}

#[test]
fn the_dirty_tree_precondition_exits_70_against_an_injected_scratch_root_only() {
    let scratch = std::env::temp_dir().join(format!(
        "test-mermaid-accuracy-dirty-{}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&scratch);
    std::fs::create_dir_all(&scratch).unwrap();
    std::fs::write(scratch.join(".setup-dev-env.started"), "token-a\n").unwrap();
    // .finished absent -> dirty.

    let output = run_against(&scratch);
    assert_eq!(output.status.code(), Some(70));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("started and never finished"), "{stderr}");
    let _ = std::fs::remove_dir_all(&scratch);
}
