// MODE: DEV
//! Integration tests for plan-root's opt-in plans-same-repo-branch mode
//! (W29): a pre-existing recognized plans-branch worktree is used silently
//! with no prompt; a pre-seeded config resolves (or opts out) with no new
//! prompt, for both plans_storage values; a plan created via the real
//! create-plan binary inside the worktree lands a real commit there with a
//! non-empty PLAN_SNAPSHOT_REPO (W47); a second real plan-mutating command
//! run afterward lands its own new commit (W55, AR-54). Genuinely
//! interactive accept/decline of the new question need a real pty and are
//! exercised for real in goal 08's own W35/W36, not here (mirroring
//! AR-31/AR-32/AR-33).

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

const TEST_USER: &str = "plan-root-flow-test-user";

fn run_git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .status()
        .expect("git runs");
    assert!(status.success(), "git {args:?} failed in {}", dir.display());
}

fn init_project(dir: &Path) {
    run_git(dir, &["init", "-q"]);
    run_git(
        dir,
        &[
            "-c",
            "user.name=test",
            "-c",
            "user.email=test@example.com",
            "commit",
            "-q",
            "--allow-empty",
            "-m",
            "init",
        ],
    );
}

/// The canonical plans-branch worktree path `plan-root` itself would
/// compute for `project`, given no git remote (the user/projectdir
/// fallback shape).
fn candidate_plans_branch_root(xdg_config_home: &Path, project: &Path) -> PathBuf {
    let project_dir = project.file_name().unwrap().to_str().unwrap();
    xdg_config_home
        .join("tsch-ai-skills")
        .join("plans-branch")
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

fn seed_tsch_config(xdg_config_home: &Path, project: &Path, storage: &str, branch: Option<&str>) {
    let path = config_path(xdg_config_home, project);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    let mut object = serde_json::json!({ "plans_storage": storage });
    if let Some(branch) = branch {
        object["plans_branch"] = serde_json::Value::String(branch.to_string());
    }
    std::fs::write(&path, serde_json::to_string_pretty(&object).unwrap()).unwrap();
}

/// Creates a recognized cone-mode plans-branch worktree directly against
/// real git -- the way create_sparse_worktree (W03) creates one for real --
/// rather than through plan-root's own interactive prompt, which needs a
/// real pty (goal 08's own job, mirroring AR-31/32/33).
fn make_recognized_worktree(project: &Path, candidate: &Path, branch: &str) {
    run_git(
        project,
        &["worktree", "add", "-b", branch, candidate.to_str().unwrap()],
    );
    run_git(candidate, &["sparse-checkout", "init"]);
    run_git(candidate, &["sparse-checkout", "set", ".plans"]);
}

/// Walks up from this test binary's own path to the workspace's shared
/// target/{debug,release} directory, where a sibling binary built alongside
/// this crate lands -- mirroring planning-server/tests/integration.rs's own
/// `sibling_bin_dir`.
fn sibling_bin_dir() -> PathBuf {
    let mut dir = std::env::current_exe().expect("current test binary path");
    dir.pop();
    if dir.file_name().is_some_and(|name| name == "deps") {
        dir.pop();
    }
    dir
}

/// Builds `name` into `bin_dir` if it is not there yet -- create-plan and
/// add-goal are separate packages with no Cargo dependency edge on this
/// crate, so `cargo test --workspace` gives no ordering guarantee that they
/// finish building first (mirroring planning-server/tests/integration.rs's
/// own `ensure_built`, without that crate's own further-sibling special
/// case, which neither create-plan nor add-goal needs).
fn ensure_built(bin_dir: &Path, name: &str) -> PathBuf {
    let program = bin_dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    if program.is_file() {
        return program;
    }
    static BUILDING: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _one_at_a_time = BUILDING.lock().unwrap_or_else(|p| p.into_inner());
    if program.is_file() {
        return program;
    }
    let mut cmd = Command::new(env!("CARGO"));
    cmd.arg("build").arg("-p").arg(name);
    if let Some(triple) = bin_dir
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .filter(|name| *name != "target")
    {
        cmd.arg("--target").arg(triple);
    }
    let mut workspace_root = bin_dir.to_path_buf();
    loop {
        let popped = workspace_root.file_name().map(|n| n.to_os_string());
        if !workspace_root.pop() {
            panic!("bin_dir has no 'target' ancestor: {}", bin_dir.display());
        }
        if popped.as_deref() == Some(std::ffi::OsStr::new("target")) {
            break;
        }
    }
    let output = cmd
        .current_dir(&workspace_root)
        .output()
        .unwrap_or_else(|error| panic!("could not build {name}: {error}"));
    assert!(
        output.status.success(),
        "building {name} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        program.is_file(),
        "{name} still missing at {} after building it",
        program.display()
    );
    program
}

fn commit_count(dir: &Path) -> u32 {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-list", "--count", "HEAD"])
        .output()
        .expect("git runs");
    String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse()
        .expect("a commit count")
}

fn toplevel_of(dir: &Path) -> String {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .expect("git runs");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

/// Runs the real compiled `plan-root` binary with an isolated
/// XDG_CONFIG_HOME and a fixed USER (so the fallback owner/repo-less shape
/// is deterministic), cwd at `project`, and stdin on `/dev/null` -- not
/// piped, not a pty. Every assertion below that claims "no [new] prompt"
/// checks stderr directly for the new plans-branch prompt's own text (the
/// existing first question's own non-interactive note is expected,
/// pre-existing, unrelated chatter and is deliberately not asserted away).
fn run(project: &Path, xdg_config_home: &Path, args: &[&str]) -> (i32, String, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_plan-root"))
        .args(args)
        .current_dir(project)
        .env("XDG_CONFIG_HOME", xdg_config_home)
        .env("USER", TEST_USER)
        .env_remove("USERNAME")
        .env_remove("PLANS_ROOT")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("plan-root runs");
    (
        output.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&output.stdout).trim().to_string(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn a_pre_existing_recognized_plans_branch_worktree_is_used_silently() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    let home = tempfile::tempdir().unwrap();
    let candidate = candidate_plans_branch_root(home.path(), project.path());
    make_recognized_worktree(project.path(), &candidate, "plans");

    let (code, out, err) = run(project.path(), home.path(), &["resolve"]);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(
        err.is_empty(),
        "the early short-circuit never reaches choose_root: {err}"
    );
    assert_eq!(out, candidate.join(".plans").display().to_string());
}

#[test]
fn a_preseeded_same_repo_branch_config_creates_and_resolves_with_no_new_prompt() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    let home = tempfile::tempdir().unwrap();
    let candidate = candidate_plans_branch_root(home.path(), project.path());
    assert!(!candidate.exists(), "nothing created yet");
    seed_tsch_config(
        home.path(),
        project.path(),
        "same-repo-branch",
        Some("a-custom-name"),
    );

    let (code, out, err) = run(project.path(), home.path(), &["resolve"]);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(
        !err.contains("Keep plan history"),
        "no new prompt output: {err}"
    );
    assert_eq!(out, candidate.join(".plans").display().to_string());
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
}

#[test]
fn a_preseeded_separate_repo_config_never_disturbs_todays_default() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    let home = tempfile::tempdir().unwrap();
    seed_tsch_config(home.path(), project.path(), "separate-repo", None);

    let (code, out, err) = run(project.path(), home.path(), &["resolve"]);
    assert_eq!(code, 0, "stderr: {err}");
    assert!(
        !err.contains("Keep plan history"),
        "no new prompt output: {err}"
    );
    // The resolver canonicalizes the cwd (via project_root_for) before
    // joining .plans onto it, so the expected side must go through the same
    // canonicalization -- otherwise this compares a resolved path against
    // the tempdir's own pre-canonicalization spelling, which differ wherever
    // the OS hands back a non-canonical temp path (macOS's /var ->
    // /private/var symlink, a Windows 8.3 short name).
    let canonical_project = planning_core::canonicalize(project.path()).unwrap();
    assert_eq!(out, canonical_project.join(".plans").display().to_string());

    let candidate = candidate_plans_branch_root(home.path(), project.path());
    assert!(!candidate.exists(), "no plans-branch worktree ever created");
}

#[test]
fn a_plan_created_inside_the_worktree_lands_a_real_commit_with_a_nonempty_snapshot_repo() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    let home = tempfile::tempdir().unwrap();
    let candidate = candidate_plans_branch_root(home.path(), project.path());
    make_recognized_worktree(project.path(), &candidate, "plans");

    let (code, resolved, err) = run(project.path(), home.path(), &["resolve"]);
    assert_eq!(code, 0, "stderr: {err}");
    assert_eq!(resolved, candidate.join(".plans").display().to_string());

    let before = commit_count(&candidate);

    let create_output = Command::new(ensure_built(&sibling_bin_dir(), "create-plan"))
        .args(["my-first-plan", "A first plan"])
        .current_dir(project.path())
        .env("PLANS_ROOT", &resolved)
        .output()
        .expect("create-plan runs");
    assert!(
        create_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&create_output.stderr)
    );

    let plan_dir = PathBuf::from(&resolved).join("my-first-plan");
    assert!(plan_dir.join("plan-description.md").is_file());

    let after = commit_count(&candidate);
    assert!(after > before, "create-plan must land a real commit (W47)");

    let env_text = std::fs::read_to_string(plan_dir.join(".env")).unwrap();
    let snapshot_line = env_text
        .lines()
        .find(|line| line.starts_with("PLAN_SNAPSHOT_REPO="))
        .expect("PLAN_SNAPSHOT_REPO is recorded");
    let snapshot_value = snapshot_line
        .trim_start_matches("PLAN_SNAPSHOT_REPO=")
        .trim_matches('\'');
    assert!(
        !snapshot_value.is_empty(),
        "must never be empty inside a plans-branch worktree"
    );
    assert_eq!(snapshot_value, toplevel_of(&candidate));
}

#[test]
fn a_second_plan_mutating_command_lands_its_own_new_commit() {
    let project = tempfile::tempdir().unwrap();
    init_project(project.path());
    let home = tempfile::tempdir().unwrap();
    let candidate = candidate_plans_branch_root(home.path(), project.path());
    make_recognized_worktree(project.path(), &candidate, "plans");

    let (code, resolved, err) = run(project.path(), home.path(), &["resolve"]);
    assert_eq!(code, 0, "stderr: {err}");

    let create_output = Command::new(ensure_built(&sibling_bin_dir(), "create-plan"))
        .args(["second-plan", "A second plan"])
        .current_dir(project.path())
        .env("PLANS_ROOT", &resolved)
        .output()
        .expect("create-plan runs");
    assert!(create_output.status.success());
    let plan_dir = PathBuf::from(&resolved).join("second-plan");

    // A person (or agent) editing a plan file between invocations is the
    // realistic case the pre-mutation safety-net snapshot exists for --
    // right after create-plan's own commit the tree is otherwise clean, so
    // nothing would be left for a second command's own snapshot to catch.
    std::fs::write(plan_dir.join("commands.json"), "{\"touched\": true}\n").unwrap();

    let before_second = commit_count(&candidate);

    let add_goal_output = Command::new(ensure_built(&sibling_bin_dir(), "add-goal"))
        .args([
            plan_dir.to_str().unwrap(),
            "02-second-goal",
            "Second goal",
            "Outcome text",
        ])
        .current_dir(project.path())
        .output()
        .expect("add-goal runs");
    assert!(
        add_goal_output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&add_goal_output.stderr)
    );

    let after_second = commit_count(&candidate);
    assert!(
        after_second > before_second,
        "a second plan-mutating command must land its own new snapshot commit (W55, AR-54)"
    );
}
