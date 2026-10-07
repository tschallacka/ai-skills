// MODE: DEV
//! End-to-end: add -> list open -> answer -> list open/decided -> apply ->
//! list closed, against the compiled binary, the way an agent actually
//! drives it. A second test covers the other terminal path: decided ->
//! implement -> list implemented.

use std::process::Command;

fn run(home: &std::path::Path, args: &[&str]) -> (i32, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_decisions"))
        .args(args)
        .env("DECISIONS_JSON", home.join("DECISIONS.json"))
        .output()
        .expect("decisions runs");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).into_owned(),
    )
}

#[test]
fn add_list_answer_list_apply_list_round_trips_the_whole_lifecycle() {
    let home = tempfile::tempdir().expect("scratch home");
    std::fs::write(
        home.path().join("DECISIONS.json"),
        r#"{"skill":"decisions","skill_version":"2.0.0-alpha.5","comment":"t","questions":[]}"#,
    )
    .expect("seed register");

    let (code, out) = run(
        home.path(),
        &[
            "add",
            "--title",
            "Pick a strategy",
            "--option",
            "a:Yes",
            "--option",
            "b:No",
            "--priority",
            "urgent",
            "--context",
            "stubbed with a while waiting",
        ],
    );
    assert_eq!(code, 0, "add succeeds: {out}");
    let id = out.trim().to_string();
    assert_eq!(id, "Q1");

    let (code, out) = run(home.path(), &["list", "--status", "open"]);
    assert_eq!(code, 0);
    assert!(out.contains(&id), "open list shows it: {out}");

    let (code, out) = run(home.path(), &["answer", &id, "a"]);
    assert_eq!(code, 0, "answer succeeds: {out}");

    let (code, out) = run(home.path(), &["list", "--status", "open"]);
    assert_eq!(code, 0);
    assert!(!out.contains(&id), "no longer open: {out}");

    let (code, out) = run(home.path(), &["list", "--status", "decided"]);
    assert_eq!(code, 0);
    assert!(out.contains(&id), "now decided: {out}");

    let (code, out) = run(home.path(), &["apply", &id, "Went with option a"]);
    assert_eq!(code, 0, "apply succeeds: {out}");

    let (code, out) = run(home.path(), &["list", "--status", "closed"]);
    assert_eq!(code, 0);
    assert!(out.contains(&id), "now closed: {out}");

    let text = std::fs::read_to_string(home.path().join("DECISIONS.json")).unwrap();
    assert!(text.contains("\"chosen\": \"a\""));
    assert!(text.contains("\"resolution\": \"Went with option a\""));
}

#[test]
fn stub_appends_context_without_changing_status() {
    let home = tempfile::tempdir().expect("scratch home");
    std::fs::write(
        home.path().join("DECISIONS.json"),
        r#"{"skill":"decisions","skill_version":"2.0.0-alpha.5","comment":"t","questions":[]}"#,
    )
    .expect("seed register");
    let (_, out) = run(
        home.path(),
        &[
            "add", "--title", "T", "--option", "a:Yes", "--option", "b:No",
        ],
    );
    let id = out.trim().to_string();

    let (code, _) = run(home.path(), &["stub", &id, "assumed", "option", "a"]);
    assert_eq!(code, 0);

    let (code, out) = run(home.path(), &["list", "--status", "open"]);
    assert_eq!(code, 0);
    assert!(out.contains(&id), "still open after a stub: {out}");
}

#[test]
fn add_answer_implement_list_implemented_round_trips_the_other_terminal_path() {
    let home = tempfile::tempdir().expect("scratch home");
    std::fs::write(
        home.path().join("DECISIONS.json"),
        r#"{"skill":"decisions","skill_version":"2.0.0-alpha.5","comment":"t","questions":[]}"#,
    )
    .expect("seed register");

    let (_, out) = run(
        home.path(),
        &[
            "add", "--title", "Pick one", "--option", "a:Yes", "--option", "b:No",
        ],
    );
    let id = out.trim().to_string();

    let (code, _) = run(home.path(), &["implement", &id, "too early"]);
    assert_eq!(code, 65, "implementing before a pick exists is refused");

    let (code, _) = run(home.path(), &["answer", &id, "a"]);
    assert_eq!(code, 0);

    let (code, out) = run(home.path(), &["list", "--status", "decided"]);
    assert_eq!(code, 0);
    assert!(out.contains(&id), "now decided: {out}");

    let (code, _) = run(home.path(), &["implement", &id, "landed in src/thing.rs"]);
    assert_eq!(code, 0);

    let (code, out) = run(home.path(), &["list", "--status", "decided"]);
    assert_eq!(code, 0);
    assert!(!out.contains(&id), "no longer decided: {out}");

    let (code, out) = run(home.path(), &["list", "--status", "implemented"]);
    assert_eq!(code, 0);
    assert!(out.contains(&id), "now implemented: {out}");

    let text = std::fs::read_to_string(home.path().join("DECISIONS.json")).unwrap();
    assert!(text.contains("\"resolution\": \"landed in src/thing.rs\""));
}
