// MODE: DEV
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("create-plan-flow-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    // Without the `\\?\` prefix Windows adds: create-plan writes the same
    // prefix-free form, and compares against it.
    planning_core::canonicalize(&dir).unwrap()
}

fn git(dir: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=t", "-c", "user.email=t@t"])
        .args(args)
        .output()
        .expect("run git")
}

// A project's `.plans`, git-ignored by the project: the plan gets its OWN
// repository at the plans root, and that repository is what the manifest pins
// for pre-mutation snapshots. Creating the plan used to leave `.plans` without
// a repository of its own (the `add` ran against the enclosing project and
// refused the ignored path) and pin nothing, so an overwritten paragraph was
// unrecoverable.
#[test]
fn a_gitignored_plans_root_gets_its_own_repository_and_pins_it() {
    let project = scratch("ignored-root");
    assert!(git(&project, &["init", "-q"]).status.success());
    fs::write(project.join(".gitignore"), "/.plans\n").unwrap();
    assert!(git(&project, &["add", "-A"]).status.success());
    assert!(git(&project, &["commit", "-qm", "init"]).status.success());
    let plans = project.join(".plans");
    fs::create_dir_all(&plans).unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_create-plan"))
        .args(["plan-a", "Snapshot probe"])
        .current_dir(&project)
        .env("PLANS_ROOT", &plans)
        .env("PLAN_NONINTERACTIVE", "1")
        .output()
        .expect("run create-plan");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );

    assert!(
        plans.join(".git").exists(),
        "no repository at the plans root"
    );
    let log = git(&plans, &["log", "--oneline"]);
    assert!(
        String::from_utf8_lossy(&log.stdout).contains("plan: initial structure"),
        "{}",
        String::from_utf8_lossy(&log.stdout)
    );
    // Read back the way the manifest is read (quoting and all), not compared
    // as text: a Windows path is written with each backslash escaped.
    let plan = plans.join("plan-a");
    assert_eq!(
        planning_core::snapshot_repo(&plan).as_deref(),
        Some(plans.as_path()),
        "{}",
        fs::read_to_string(plan.join(".env")).unwrap()
    );

    let _ = fs::remove_dir_all(&project);
}

fn git_config(dir: &Path, key: &str) -> String {
    String::from_utf8_lossy(&git(dir, &["config", "--local", "--get", key]).stdout)
        .trim()
        .to_string()
}

fn run_create_plan(cwd: &Path, plans_root: &Path) {
    let output = Command::new(env!("CARGO_BIN_EXE_create-plan"))
        .args(["plan-a", "Maintenance probe"])
        .current_dir(cwd)
        .env("PLANS_ROOT", plans_root)
        .env("PLAN_NONINTERACTIVE", "1")
        .output()
        .expect("run create-plan");
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

// Git runs automatic maintenance detached after a commit, so it outlives the
// command that committed and creates and removes files under `.git` while the
// plan is being read, copied or removed. A repository create-plan makes for a
// plan runs it in the foreground instead, where it finishes before the commit
// returns.
#[test]
fn a_repository_created_for_a_plan_runs_git_maintenance_in_the_foreground() {
    let plans = scratch("maintenance-new").join("plans");
    fs::create_dir_all(&plans).unwrap();
    run_create_plan(&plans, &plans);

    let plan_repo = if plans.join(".git").exists() {
        plans.clone()
    } else {
        plans.join("plan-a")
    };
    for key in ["gc.autoDetach", "maintenance.autoDetach"] {
        assert_eq!(git_config(&plan_repo, key), "false", "{key}");
    }

    let _ = fs::remove_dir_all(plans.parent().unwrap());
}

// A project's own repository is the user's: only a repository this tool
// creates gets the setting.
#[test]
fn an_existing_project_repository_keeps_its_own_maintenance_settings() {
    let project = scratch("maintenance-existing");
    assert!(git(&project, &["init", "-q"]).status.success());
    let plans = project.join(".plans");
    fs::create_dir_all(&plans).unwrap();
    run_create_plan(&project, &plans);

    for key in ["gc.autoDetach", "maintenance.autoDetach"] {
        assert_eq!(
            git_config(&project, key),
            "",
            "{key} was set on the project"
        );
    }

    let _ = fs::remove_dir_all(&project);
}
