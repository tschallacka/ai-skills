// MODE: DEV

use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn repo_root() -> PathBuf {
    // CARGO_MANIFEST_DIR is src/generate-postmortem; the repo root is two
    // levels up.
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf()
}

fn run(plan_dir: &std::path::Path, output: &std::path::Path) -> std::process::Output {
    Command::new(env!("CARGO_BIN_EXE_generate-postmortem"))
        .arg(plan_dir)
        .arg("--output")
        .arg(output)
        .output()
        .expect("binary runs")
}

#[test]
fn a_synthetic_history_with_mixed_field_presence_renders_exactly_as_expected() {
    let scratch = tempfile::tempdir().expect("tempdir");
    let plan_dir = scratch.path().join("plan");
    fs::create_dir_all(&plan_dir).unwrap();
    fs::write(
        plan_dir.join("adversarial-review-history.md"),
        "\n## Cycle 1\n\n- Reviewer session: sess-a1\n- Elapsed: 5min\n- Cost signal: 2 findings this cycle\n- Tokens: 1200\n\n| ID | Missing or over-broad item | Required plan change | Status | Work unit |\n|---|---|---|---|---|\n| AR-01 | x | y | resolved | W01 |\n| AR-02 | x | y | resolved | W01 |\n\n## Cycle 2\n\n| ID | Missing or over-broad item | Required plan change | Status | Work unit |\n|---|---|---|---|---|\n| AR-03 | x | y | resolved | W02 |\n",
    )
    .unwrap();

    let output = scratch.path().join("out.md");
    let result = run(&plan_dir, &output);
    assert!(result.status.success(), "{result:?}");
    let rendered = fs::read_to_string(&output).unwrap();

    assert!(rendered.contains("| 1 | sess-a1 | 5min | 2 findings this cycle | 1200 | 2 |"));
    assert!(
        rendered.contains("| 2 | not reported | not reported | not reported | not reported | 1 |")
    );
    assert!(rendered.contains("Total cycles: 2"));
    assert!(rendered.contains("Total findings: 3"));
    assert!(rendered.contains("Total tokens: 1200 (partial total, 1 of 2 cycles reported)"));
}

fn count_cycle_headings(text: &str) -> usize {
    text.lines()
        .filter(|line| line.starts_with("## Cycle "))
        .count()
}

fn assert_matches_real_history(relative_plan_dir: &str) {
    let root = repo_root();
    let plan_dir = root.join(relative_plan_dir);
    let history_file = plan_dir.join("adversarial-review-history.md");
    // These plans live under .plans/, which is git-ignored, so they exist only
    // on the maintainer's machine; a fresh checkout has nothing to compare.
    if !history_file.is_file() {
        eprintln!(
            "skipping: {} is a local-only plan, absent from this checkout",
            history_file.display()
        );
        return;
    }
    let history_text = fs::read_to_string(&history_file)
        .unwrap_or_else(|error| panic!("reading {}: {error}", history_file.display()));
    let expected_cycles = count_cycle_headings(&history_text);

    let scratch = tempfile::tempdir().expect("tempdir");
    let output = scratch.path().join("out.md");
    let result = run(&plan_dir, &output);
    assert!(result.status.success(), "{result:?}");
    let rendered = fs::read_to_string(&output).unwrap();

    assert!(rendered.contains(&format!("Total cycles: {expected_cycles}")));
    // Real-world state today (per goal 02-postmortem-generator section 2.1/8.1):
    // no cycle in this repository's own real history files carries a
    // self-reported field. If this ever stops being true, that is real cost
    // data finally showing up -- update this assertion, do not "fix" it back.
    assert!(
        !rendered.contains("not reported")
            || rendered.matches("not reported").count() >= expected_cycles * 4 - 1,
        "expected almost every self-reported field slot to read 'not reported' for {relative_plan_dir}"
    );
}

#[test]
fn matches_the_real_bash_to_rust_conversion_history() {
    assert_matches_real_history(".plans/bash-to-rust-conversion");
}

#[test]
fn matches_the_real_persona_profile_migration_history() {
    assert_matches_real_history(".plans/persona-profile-migration");
}

#[test]
fn the_wired_shell_oracle_matches_the_compiled_binary_directly() {
    let root = repo_root();
    let oracle = root.join("planning/scripts/generate-postmortem.sh");
    // Pinned explicitly to a scratch directory holding exactly the binary
    // under test. An existing-but-incomplete shared install directory
    // (~/.config/tsch-ai-skills/bin) would otherwise shadow it via
    // plan_bin_dir()'s tier-2 lookup even when it lacks this specific binary,
    // since that tier only checks the directory exists -- and reading the
    // repo's own bin/<triple> would need a triple hardcoded here and a
    // `./setup-dev-env.sh` run a fresh checkout does not have.
    let staged = tempfile::tempdir().expect("tempdir");
    let bin_root = staged.path().join("bin");
    fs::create_dir_all(&bin_root).unwrap();
    fs::copy(
        env!("CARGO_BIN_EXE_generate-postmortem"),
        bin_root.join("generate-postmortem"),
    )
    .unwrap();

    let help = Command::new(&oracle)
        .arg("--help")
        .env("AI_SKILLS_BIN_ROOT", &bin_root)
        .output()
        .expect("oracle runs");
    assert!(help.status.success(), "{help:?}");

    let scratch = tempfile::tempdir().expect("tempdir");
    let plan_dir = scratch.path().join("plan");
    fs::create_dir_all(&plan_dir).unwrap();
    fs::write(
        plan_dir.join("adversarial-review-history.md"),
        "\n## Cycle 1\n\n- Reviewer session: sess-a1\n- Elapsed: 5min\n- Cost signal: 2 findings this cycle\n- Tokens: 1200\n\n| ID | Missing or over-broad item | Required plan change | Status | Work unit |\n|---|---|---|---|---|\n| AR-01 | x | y | resolved | W01 |\n| AR-02 | x | y | resolved | W01 |\n\n## Cycle 2\n\n| ID | Missing or over-broad item | Required plan change | Status | Work unit |\n|---|---|---|---|---|\n| AR-03 | x | y | resolved | W02 |\n",
    )
    .unwrap();
    let via_oracle = scratch.path().join("via-oracle.md");
    let via_binary = scratch.path().join("via-binary.md");

    let oracle_result = Command::new(&oracle)
        .arg(&plan_dir)
        .arg("--output")
        .arg(&via_oracle)
        .env("AI_SKILLS_BIN_ROOT", &bin_root)
        .output()
        .expect("oracle runs");
    assert!(oracle_result.status.success(), "{oracle_result:?}");

    let binary_result = run(&plan_dir, &via_binary);
    assert!(binary_result.status.success(), "{binary_result:?}");

    assert_eq!(
        fs::read_to_string(&via_oracle).unwrap(),
        fs::read_to_string(&via_binary).unwrap()
    );
}
