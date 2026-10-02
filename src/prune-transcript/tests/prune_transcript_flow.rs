// MODE: DEV
//! End-to-end regression: drive the real built `prune-transcript` binary
//! against a small synthetic JSONL fixture, the same way an operator would
//! from the shell.

use serde_json::{json, Value};
use std::fs;
use std::path::PathBuf;
use std::process::{Command, Output};

fn line(value: Value) -> String {
    serde_json::to_string(&value).unwrap()
}

fn fixture() -> String {
    let user_line = line(json!({
        "uuid": "u1", "parentUuid": null, "type": "user",
        "message": {"role": "user", "content": "hello"}
    }));
    let assistant_line = line(json!({
        "uuid": "a1", "parentUuid": "u1", "type": "assistant",
        "message": {"role": "assistant", "content": "hi there"}
    }));
    let skill_listing_line = line(json!({
        "uuid": "a2", "parentUuid": "a1", "type": "attachment",
        "attachment": {"type": "skill_listing", "content": "S".repeat(1000)}
    }));
    let prompt_snapshot_line = line(json!({
        "uuid": "a3", "parentUuid": "a2", "type": "attachment",
        "attachment": {"type": "prompt_snapshot", "systemPrompt": ["P".repeat(400), "Q".repeat(400)]}
    }));
    let invoked_skills_line = line(json!({
        "uuid": "a4", "parentUuid": "a3", "type": "attachment",
        "attachment": {"type": "invoked_skills", "skills": [
            {"name": "todo", "path": "/skills/todo/SKILL.md", "content": "T".repeat(800)}
        ]}
    }));
    let malformed_line = "{this is not valid json, but mentions skill_listing".to_owned();

    format!(
        "{user_line}\n{assistant_line}\n{skill_listing_line}\n{prompt_snapshot_line}\n{invoked_skills_line}\n{malformed_line}\n"
    )
}

fn run(binary: &str, args: &[&str]) -> Output {
    Command::new(binary)
        .args(args)
        .output()
        .expect("failed to spawn prune-transcript")
}

#[test]
fn shrinks_known_categories_and_passes_everything_else_through() {
    let binary = env!("CARGO_BIN_EXE_prune-transcript");
    let dir = tempfile::tempdir().unwrap();
    let input_path: PathBuf = dir.path().join("input.jsonl");
    let output_path: PathBuf = dir.path().join("output.jsonl");

    let input = fixture();
    fs::write(&input_path, &input).unwrap();

    let output = run(
        binary,
        &[
            input_path.to_str().unwrap(),
            "--out",
            output_path.to_str().unwrap(),
        ],
    );
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let produced = fs::read_to_string(&output_path).unwrap();
    let input_lines: Vec<&str> = input.trim_end_matches('\n').split('\n').collect();
    let output_lines: Vec<&str> = produced.trim_end_matches('\n').split('\n').collect();
    assert_eq!(
        input_lines.len(),
        output_lines.len(),
        "line count must be preserved"
    );

    // user/assistant lines and the malformed line pass through byte-identical.
    assert_eq!(output_lines[0], input_lines[0]);
    assert_eq!(output_lines[1], input_lines[1]);
    assert_eq!(
        output_lines[5], input_lines[5],
        "malformed line must be untouched"
    );

    let skill_listing: Value = serde_json::from_str(output_lines[2]).unwrap();
    let content = skill_listing["attachment"]["content"].as_str().unwrap();
    assert!(content.starts_with("[pruned:prune-transcript"));
    assert!(content.contains("1000 bytes"));
    assert!(content.len() < 1000);

    let prompt_snapshot: Value = serde_json::from_str(output_lines[3]).unwrap();
    let chunks = prompt_snapshot["attachment"]["systemPrompt"]
        .as_array()
        .unwrap();
    assert_eq!(chunks.len(), 2, "array length must be preserved");
    for chunk in chunks {
        assert!(chunk
            .as_str()
            .unwrap()
            .starts_with("[pruned:prune-transcript"));
    }

    let invoked_skills: Value = serde_json::from_str(output_lines[4]).unwrap();
    let skill = &invoked_skills["attachment"]["skills"][0];
    assert_eq!(skill["name"], "todo");
    assert_eq!(skill["path"], "/skills/todo/SKILL.md");
    let skill_content = skill["content"].as_str().unwrap();
    assert!(skill_content.starts_with("[pruned:prune-transcript"));
    assert!(skill_content.contains("/skills/todo/SKILL.md"));

    // A second run against the tool's own output must be a complete no-op.
    let second_output_path = dir.path().join("second.jsonl");
    let second = run(
        binary,
        &[
            output_path.to_str().unwrap(),
            "--out",
            second_output_path.to_str().unwrap(),
        ],
    );
    assert!(second.status.success());
    let twice = fs::read_to_string(&second_output_path).unwrap();
    assert_eq!(twice, produced, "a second pass must change nothing further");

    // --verify between the original input and the pruned output must pass.
    let verify = run(
        binary,
        &[
            "--verify",
            input_path.to_str().unwrap(),
            output_path.to_str().unwrap(),
        ],
    );
    assert!(
        verify.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&verify.stdout),
        String::from_utf8_lossy(&verify.stderr)
    );
}

#[test]
fn dry_run_writes_nothing() {
    let binary = env!("CARGO_BIN_EXE_prune-transcript");
    let dir = tempfile::tempdir().unwrap();
    let input_path: PathBuf = dir.path().join("input.jsonl");
    fs::write(&input_path, fixture()).unwrap();

    let output = run(binary, &[input_path.to_str().unwrap(), "--dry-run"]);
    assert!(output.status.success());
    assert!(!dir.path().join("output.jsonl").exists());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("dry run"));
}

#[test]
fn in_place_rewrites_the_input_file_atomically() {
    let binary = env!("CARGO_BIN_EXE_prune-transcript");
    let dir = tempfile::tempdir().unwrap();
    let input_path: PathBuf = dir.path().join("input.jsonl");
    let original = fixture();
    fs::write(&input_path, &original).unwrap();

    let output = run(binary, &[input_path.to_str().unwrap(), "--in-place"]);
    assert!(
        output.status.success(),
        "stdout: {}\nstderr: {}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let rewritten = fs::read_to_string(&input_path).unwrap();
    assert_ne!(
        rewritten, original,
        "in-place run must have shrunk something"
    );
    assert!(rewritten.contains("[pruned:prune-transcript"));

    // No leftover temp file.
    let leftovers: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|name| name.contains(".tmp-"))
        .collect();
    assert!(leftovers.is_empty(), "leftover temp files: {leftovers:?}");
}
