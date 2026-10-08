// MODE: DEV
//! End-to-end: add -> list open -> answer -> list open/decided -> apply ->
//! list closed, against the compiled binary, the way an agent actually
//! drives it. A second test covers the other terminal path: decided ->
//! implement -> list implemented.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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

// --- Registers-worktree resolution (W17): a pre-existing recognized
// worktree is used silently with no prompt; --file/DECISIONS_JSON always
// wins regardless; `decisions resolve-path` prints the same path a mutating
// command actually used; a pre-seeded tsch config resolves (or opts out)
// with no prompt at all, for both registers_access values. Genuinely
// interactive accept/decline scenarios need a real pty and are exercised
// for real in goal 08's own W30/W32, not here (AR-33, mirroring AR-31/32).

const TEST_USER: &str = "cli-flow-test-user";

fn run_git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .status()
        .expect("git runs");
    assert!(status.success(), "git {args:?} failed in {}", dir.display());
}

/// A real git project with one commit that already carries a seed
/// DECISIONS.json, so a worktree later forked from HEAD (no origin/<branch>
/// exists in these tests) checks one out too.
fn init_registers_project(dir: &Path) {
    run_git(dir, &["init", "-q"]);
    std::fs::write(
        dir.join("DECISIONS.json"),
        r#"{"skill":"decisions","skill_version":"2.0.0-alpha.5","comment":"t","questions":[]}"#,
    )
    .expect("seed register");
    run_git(dir, &["add", "DECISIONS.json"]);
    run_git(
        dir,
        &[
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-q",
            "-m",
            "init",
        ],
    );
}

/// The canonical registers-worktree path `decisions` itself would compute
/// for `project`, given no git remote (so the user/projectdir fallback shape).
fn candidate_registers_root(xdg_config_home: &Path, project: &Path) -> PathBuf {
    let project_dir = project.file_name().unwrap().to_str().unwrap();
    xdg_config_home
        .join("tsch-ai-skills")
        .join("registers")
        .join(TEST_USER)
        .join(project_dir)
}

fn config_path(xdg_config_home: &Path, project: &Path) -> PathBuf {
    let project_dir = project.file_name().unwrap().to_str().unwrap();
    xdg_config_home
        .join("tsch-ai-skills")
        .join("config")
        .join(TEST_USER)
        .join(format!("{project_dir}.json"))
}

fn seed_tsch_config(xdg_config_home: &Path, project: &Path, access: &str, branch: Option<&str>) {
    let path = config_path(xdg_config_home, project);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut object = serde_json::json!({ "registers_access": access });
    if let Some(branch) = branch {
        object["registers_branch"] = serde_json::Value::String(branch.to_string());
    }
    std::fs::write(&path, serde_json::to_string_pretty(&object).unwrap()).unwrap();
}

/// Creates a recognized registers worktree directly against real git, the
/// way goal 06 adopts one by hand -- not through `decisions`' own
/// interactive prompt, which needs a real pty (goal 08's own job, AR-33).
fn make_recognized_worktree(project: &Path, candidate: &Path, branch: &str) {
    run_git(
        project,
        &["worktree", "add", "-b", branch, candidate.to_str().unwrap()],
    );
    run_git(candidate, &["sparse-checkout", "init", "--no-cone"]);
    run_git(
        candidate,
        &[
            "sparse-checkout",
            "set",
            "/BUGS.json",
            "/TODO.json",
            "/DECISIONS.json",
        ],
    );
}

/// Runs the real compiled `decisions` binary with an isolated
/// XDG_CONFIG_HOME and a fixed USER (so the fallback owner/repo-less shape
/// is deterministic), cwd at `project`, and stdin on `/dev/null` -- not
/// piped, not a pty, so a stray prompt attempt reads nothing rather than
/// blocking. Every assertion below that claims "no prompt" checks stderr
/// directly for the prompt's own text, which is the stronger, directly
/// observable half of that claim.
fn run_resolution(
    project: &Path,
    xdg_config_home: &Path,
    args: &[&str],
    extra_env: &[(&str, &str)],
) -> (i32, String, String) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_decisions"));
    command
        .args(args)
        .current_dir(project)
        .env("XDG_CONFIG_HOME", xdg_config_home)
        .env("USER", TEST_USER)
        .env_remove("USERNAME")
        .env_remove("DECISIONS_JSON")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in extra_env {
        command.env(key, value);
    }
    let output = command.output().expect("decisions runs");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).trim().to_string(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn a_pre_existing_recognized_worktree_is_used_silently() {
    let project = tempfile::tempdir().unwrap();
    init_registers_project(project.path());
    let home = tempfile::tempdir().unwrap();
    let candidate = candidate_registers_root(home.path(), project.path());
    make_recognized_worktree(project.path(), &candidate, "registers");

    let (code, out, err) = run_resolution(
        project.path(),
        home.path(),
        &[
            "add", "--title", "T", "--option", "a:Yes", "--option", "b:No",
        ],
        &[],
    );
    assert_eq!(code, 0, "stderr: {err}");
    assert!(err.is_empty(), "no prompt output: {err}");
    assert_eq!(out, "Q1");

    let text = std::fs::read_to_string(candidate.join("DECISIONS.json")).unwrap();
    assert!(
        text.contains("Q1"),
        "the worktree's own file got the new entry"
    );
}

#[test]
fn a_worktree_on_the_registers_branch_elsewhere_is_recognised_from_inside_it_with_no_prompt() {
    // B404: a worktree on the registers branch at some location OTHER than
    // the fixed candidate path registers_scoped_root itself would compute --
    // exactly MAINTAINER.md 1.14's own prescribed location, which has
    // nothing to do with tsch-ai-skills/registers/<owner>/<repo>. Running
    // FROM INSIDE it must recognize it on the spot, not offer to create a
    // second, colliding worktree at the fixed candidate path.
    let project = tempfile::tempdir().unwrap();
    init_registers_project(project.path());
    let home = tempfile::tempdir().unwrap();
    let elsewhere = tempfile::tempdir().unwrap();
    let worktree = elsewhere.path().join("registers-worktree");
    make_recognized_worktree(project.path(), &worktree, "registers");

    let candidate = candidate_registers_root(home.path(), project.path());
    assert!(
        !candidate.exists(),
        "the fixed scoped-root candidate was never created"
    );

    let (code, out, err) = run_resolution(
        &worktree,
        home.path(),
        &[
            "add", "--title", "T", "--option", "a:Yes", "--option", "b:No",
        ],
        &[],
    );
    assert_eq!(code, 0, "stderr: {err}");
    assert!(err.is_empty(), "no prompt output: {err}");
    assert_eq!(out, "Q1");

    let text = std::fs::read_to_string(worktree.join("DECISIONS.json")).unwrap();
    assert!(
        text.contains("Q1"),
        "the worktree's own file got the new entry"
    );
    assert!(
        !candidate.exists(),
        "no second worktree was ever offered or created"
    );
}

#[test]
fn resolve_path_reports_the_same_path_a_mutating_command_actually_used() {
    let project = tempfile::tempdir().unwrap();
    init_registers_project(project.path());
    let home = tempfile::tempdir().unwrap();
    let candidate = candidate_registers_root(home.path(), project.path());
    make_recognized_worktree(project.path(), &candidate, "registers");

    let (code, _out, err) = run_resolution(
        project.path(),
        home.path(),
        &[
            "add", "--title", "T", "--option", "a:Yes", "--option", "b:No",
        ],
        &[],
    );
    assert_eq!(code, 0, "stderr: {err}");

    let (code, out, err) = run_resolution(project.path(), home.path(), &["resolve-path"], &[]);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(err.is_empty());
    assert_eq!(out, candidate.join("DECISIONS.json").display().to_string());
}

#[test]
fn an_explicit_override_wins_over_a_recognized_worktree() {
    let project = tempfile::tempdir().unwrap();
    init_registers_project(project.path());
    let home = tempfile::tempdir().unwrap();
    let candidate = candidate_registers_root(home.path(), project.path());
    make_recognized_worktree(project.path(), &candidate, "registers");

    let override_path = project.path().join("elsewhere.json");
    std::fs::write(
        &override_path,
        r#"{"skill":"decisions","skill_version":"2.0.0-alpha.5","comment":"t","questions":[]}"#,
    )
    .unwrap();
    let override_display = override_path.display().to_string();

    let (code, out, err) = run_resolution(
        project.path(),
        home.path(),
        &[
            "add",
            "--title",
            "T",
            "--option",
            "a:Yes",
            "--option",
            "b:No",
            "--file",
            override_path.to_str().unwrap(),
        ],
        &[],
    );
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(out, "Q1");
    let text = std::fs::read_to_string(&override_path).unwrap();
    assert!(text.contains("Q1"));

    let (code, out, err) = run_resolution(
        project.path(),
        home.path(),
        &["resolve-path", "--file", override_path.to_str().unwrap()],
        &[],
    );
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(out, override_display);

    let (code, out, err) = run_resolution(
        project.path(),
        home.path(),
        &["resolve-path"],
        &[("DECISIONS_JSON", override_path.to_str().unwrap())],
    );
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(out, override_display);
}

#[test]
fn a_preseeded_dedicated_worktree_config_creates_and_resolves_with_no_prompt() {
    let project = tempfile::tempdir().unwrap();
    init_registers_project(project.path());
    let home = tempfile::tempdir().unwrap();
    let candidate = candidate_registers_root(home.path(), project.path());
    assert!(!candidate.exists(), "nothing created yet");
    seed_tsch_config(
        home.path(),
        project.path(),
        "dedicated-worktree",
        Some("a-custom-name"),
    );

    let (code, out, err) = run_resolution(
        project.path(),
        home.path(),
        &[
            "add", "--title", "T", "--option", "a:Yes", "--option", "b:No",
        ],
        &[],
    );
    assert_eq!(code, 0, "stderr: {err}");
    assert!(err.is_empty(), "no prompt output at all: {err}");
    assert_eq!(out, "Q1");
    assert!(candidate.is_dir(), "the worktree was actually created");

    let text = std::fs::read_to_string(candidate.join("DECISIONS.json")).unwrap();
    assert!(text.contains("Q1"));

    let branch = Command::new("git")
        .arg("-C")
        .arg(&candidate)
        .args(["branch", "--show-current"])
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&branch.stdout).trim(),
        "a-custom-name",
        "created on the configured branch name, not the default"
    );

    let (code, out, err) = run_resolution(project.path(), home.path(), &["resolve-path"], &[]);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(err.is_empty());
    assert_eq!(out, candidate.join("DECISIONS.json").display().to_string());
}

#[test]
fn a_preseeded_main_checkout_config_never_prompts_and_stays_on_the_bare_file() {
    let project = tempfile::tempdir().unwrap();
    init_registers_project(project.path());
    let home = tempfile::tempdir().unwrap();
    seed_tsch_config(home.path(), project.path(), "main-checkout", None);

    let (code, out, err) = run_resolution(
        project.path(),
        home.path(),
        &[
            "add", "--title", "T", "--option", "a:Yes", "--option", "b:No",
        ],
        &[],
    );
    assert_eq!(code, 0, "stderr: {err}");
    assert!(err.is_empty(), "no prompt output at all: {err}");
    assert_eq!(out, "Q1");

    let text = std::fs::read_to_string(project.path().join("DECISIONS.json")).unwrap();
    assert!(text.contains("Q1"));

    let candidate = candidate_registers_root(home.path(), project.path());
    assert!(!candidate.exists(), "no worktree ever created");

    let (code, out, err) = run_resolution(project.path(), home.path(), &["resolve-path"], &[]);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(err.is_empty());
    assert_eq!(out, "DECISIONS.json");
}
