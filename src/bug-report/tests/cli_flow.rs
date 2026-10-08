// MODE: DEV
//! Integration tests for bugs' registers-worktree resolution (W09): a
//! pre-existing recognized worktree is used silently with no prompt;
//! --file/BUGS_JSON always wins regardless; `bugs resolve-path` prints the
//! same path a mutating command actually used; a pre-seeded tsch config
//! resolves (or opts out) with no prompt at all, for both registers_access
//! values. Genuinely interactive accept/decline scenarios need a real pty
//! and are exercised for real in goal 08's own W30/W32, not here (AR-31).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

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

/// A real git project with one commit that already carries a seed BUGS.json,
/// so a worktree later forked from HEAD (no origin/<branch> exists in these
/// tests) checks one out too, instead of `bugs fmt` failing on a branch with
/// no register file at all.
fn init_project(dir: &Path) {
    run_git(dir, &["init", "-q"]);
    std::fs::write(
        dir.join("BUGS.json"),
        r#"{"skill":"bugs","skill_version":"2.0.0-alpha.5","comment":"t","bugs":[]}"#,
    )
    .expect("seed register");
    run_git(dir, &["add", "BUGS.json"]);
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

/// The canonical registers-worktree path `bugs` itself would compute for
/// `project`, given no git remote (so the user/projectdir fallback shape).
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

fn seed_config(xdg_config_home: &Path, project: &Path, access: &str, branch: Option<&str>) {
    let path = config_path(xdg_config_home, project);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut object = serde_json::json!({ "registers_access": access });
    if let Some(branch) = branch {
        object["registers_branch"] = serde_json::Value::String(branch.to_string());
    }
    std::fs::write(&path, serde_json::to_string_pretty(&object).unwrap()).unwrap();
}

/// Creates a recognized registers worktree directly against real git, the
/// way goal 06 adopts one by hand -- not through `bugs`' own interactive
/// prompt, which needs a real pty (goal 08's own job, AR-31).
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

/// Runs the real compiled `bugs` binary with an isolated XDG_CONFIG_HOME and
/// a fixed USER (so the fallback owner/repo-less shape is deterministic),
/// cwd at `project`, and stdin on `/dev/null` -- not piped, not a pty, so a
/// stray prompt attempt reads nothing rather than blocking. Every assertion
/// below that claims "no prompt" checks stderr directly for the prompt's own
/// text, which is the stronger, directly observable half of that claim.
fn run(
    project: &Path,
    xdg_config_home: &Path,
    args: &[&str],
    extra_env: &[(&str, &str)],
) -> (i32, String, String) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_bugs"));
    command
        .args(args)
        .current_dir(project)
        .env("XDG_CONFIG_HOME", xdg_config_home)
        .env("USER", TEST_USER)
        .env_remove("USERNAME")
        .env_remove("BUGS_JSON")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in extra_env {
        command.env(key, value);
    }
    let output = command.output().expect("bugs runs");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).trim().to_string(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn a_pre_existing_recognized_worktree_is_used_silently() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    let home = tempfile::tempdir().unwrap();
    let candidate = candidate_registers_root(home.path(), project.path());
    make_recognized_worktree(project.path(), &candidate, "registers");

    let (code, out, err) = run(project.path(), home.path(), &["fmt"], &[]);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(err.is_empty(), "no prompt output: {err}");
    assert_eq!(out, candidate.join("BUGS.json").display().to_string());
}

#[test]
fn resolve_path_reports_the_same_path_a_mutating_command_actually_used() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    let home = tempfile::tempdir().unwrap();
    let candidate = candidate_registers_root(home.path(), project.path());
    make_recognized_worktree(project.path(), &candidate, "registers");

    let (code, used_path, err) = run(project.path(), home.path(), &["fmt"], &[]);
    assert_eq!(code, 0, "stderr: {err}");

    let (code, out, err) = run(project.path(), home.path(), &["resolve-path"], &[]);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(err.is_empty());
    assert_eq!(
        out, used_path,
        "resolve-path matches what fmt actually used"
    );
}

#[test]
fn an_explicit_override_wins_over_a_recognized_worktree() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    let home = tempfile::tempdir().unwrap();
    let candidate = candidate_registers_root(home.path(), project.path());
    make_recognized_worktree(project.path(), &candidate, "registers");

    let override_path = project.path().join("elsewhere.json");
    std::fs::write(
        &override_path,
        r#"{"skill":"bugs","skill_version":"2.0.0-alpha.5","comment":"t","bugs":[]}"#,
    )
    .unwrap();
    let override_display = override_path.display().to_string();

    let (code, out, err) = run(
        project.path(),
        home.path(),
        &["fmt", "--file", override_path.to_str().unwrap()],
        &[],
    );
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(out, override_display);

    let (code, out, err) = run(
        project.path(),
        home.path(),
        &["fmt"],
        &[("BUGS_JSON", override_path.to_str().unwrap())],
    );
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(out, override_display);

    let (code, out, err) = run(
        project.path(),
        home.path(),
        &["resolve-path"],
        &[("BUGS_JSON", override_path.to_str().unwrap())],
    );
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(out, override_display);
}

#[test]
fn a_preseeded_dedicated_worktree_config_creates_and_resolves_with_no_prompt() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    let home = tempfile::tempdir().unwrap();
    let candidate = candidate_registers_root(home.path(), project.path());
    assert!(!candidate.exists(), "nothing created yet");
    seed_config(
        home.path(),
        project.path(),
        "dedicated-worktree",
        Some("a-custom-name"),
    );

    let (code, out, err) = run(project.path(), home.path(), &["fmt"], &[]);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(err.is_empty(), "no prompt output at all: {err}");
    assert_eq!(out, candidate.join("BUGS.json").display().to_string());
    assert!(candidate.is_dir(), "the worktree was actually created");

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

    let (code, out, err) = run(project.path(), home.path(), &["resolve-path"], &[]);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(err.is_empty());
    assert_eq!(out, candidate.join("BUGS.json").display().to_string());
}

#[test]
fn a_preseeded_main_checkout_config_never_prompts_and_stays_on_the_bare_file() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    let home = tempfile::tempdir().unwrap();
    seed_config(home.path(), project.path(), "main-checkout", None);

    let (code, out, err) = run(project.path(), home.path(), &["fmt"], &[]);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(err.is_empty(), "no prompt output at all: {err}");
    assert_eq!(out, "BUGS.json");

    let candidate = candidate_registers_root(home.path(), project.path());
    assert!(!candidate.exists(), "no worktree ever created");

    let (code, out, err) = run(project.path(), home.path(), &["resolve-path"], &[]);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(err.is_empty());
    assert_eq!(out, "BUGS.json");
}
