// MODE: DEV
// PACKAGE: PROD
use planning_core::{project_root_for, require_safe_value, write_env_manifest};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const COMMAND: &str = "create-plan.sh";

fn usage(code: i32) -> ! {
    println!(
        "Usage: {COMMAND} <plan-name|plan-directory> <title>\n       {COMMAND} --help\n\n  <plan-directory>  an explicit path (existing behaviour).\n  <plan-name>       no '/': resolves the plans root via plan-root.sh,\n                    prompting on first use in a project."
    );
    std::process::exit(code)
}

fn die(message: impl AsRef<str>, code: i32) -> ! {
    eprintln!("{COMMAND}: {}", message.as_ref());
    std::process::exit(code)
}

fn valid_kebab(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
        && !value.starts_with('-')
        && !value.ends_with('-')
}

fn git_value(directory: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(args)
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn duplicate_steps(root: &Path) -> Vec<String> {
    let mut collisions = Vec::new();
    let Ok(plans) = fs::read_dir(root) else {
        return collisions;
    };
    for plan in plans.flatten().filter(|entry| entry.path().is_dir()) {
        let plan_name = plan.file_name().to_string_lossy().into_owned();
        let Ok(goals) = fs::read_dir(plan.path()) else {
            continue;
        };
        for goal in goals.flatten().filter(|entry| entry.path().is_dir()) {
            let steps = goal.path().join("steps");
            let Ok(entries) = fs::read_dir(&steps) else {
                continue;
            };
            let mut seen = Vec::new();
            for entry in entries.flatten() {
                let file = entry.file_name().to_string_lossy().into_owned();
                if !file.ends_with(".md") || file.ends_with("-testing.md") {
                    continue;
                }
                let Some((number, _)) = file.split_once('-') else {
                    continue;
                };
                if !number.bytes().all(|byte| byte.is_ascii_digit()) {
                    continue;
                }
                if seen.iter().any(|(n, _): &(String, String)| n == number) {
                    collisions.push(format!(
                        "{plan_name}: goal {}: {number}",
                        goal.file_name().to_string_lossy()
                    ));
                } else {
                    seen.push((number.to_string(), file));
                }
            }
        }
    }
    collisions
}

fn write_text(path: &Path, content: &str) {
    fs::write(path, content).unwrap_or_else(|error| die(error.to_string(), 64));
}

fn git_ignored(repo: &Path, path: &Path) -> bool {
    Command::new("git")
        .args(["-C"])
        .arg(repo)
        .args(["check-ignore", "-q"])
        .arg(path)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn initialise_git(plan: &Path, plans_root: &Path, bare_name: bool) {
    let existing = git_value(plan, &["rev-parse", "--show-toplevel"]).map(PathBuf::from);
    let repo = existing.as_ref().map(|top| {
        if git_ignored(top, plan) {
            plans_root.to_path_buf()
        } else {
            top.clone()
        }
    });
    if repo.is_none() {
        let init_root = if bare_name { plans_root } else { plan };
        let _ = Command::new("git")
            .args(["init", "-q"])
            .arg(init_root)
            .status();
    }
    let repo = repo.unwrap_or_else(|| {
        if bare_name {
            plans_root.to_path_buf()
        } else {
            plan.to_path_buf()
        }
    });
    let _ = Command::new("git")
        .args(["-C"])
        .arg(&repo)
        .args(["add", "-A", "--"])
        .arg(plan)
        .status();
    let _ = Command::new("git")
        .args(["-C"])
        .arg(&repo)
        .args([
            "-c",
            "user.name=plan-skill",
            "-c",
            "user.email=plan-skill@localhost",
            "commit",
            "-q",
            "-m",
            "plan: initial structure",
        ])
        .status();
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args
        .first()
        .is_some_and(|arg| arg == "--help" || arg == "-h")
    {
        usage(0)
    }
    if args.len() != 2 {
        usage(64)
    }
    let plan_arg = &args[0];
    let title = &args[1];
    let bare_name = !plan_arg.contains('/');
    let planning_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../planning");
    let (plan_dir, plans_root) = if plan_arg.contains('/') {
        let path = PathBuf::from(plan_arg);
        let root = path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();
        (path, root)
    } else {
        let project = project_root_for(None).unwrap_or_else(|message| die(message, 64));
        let root = env::var_os("PLANS_ROOT")
            .map(PathBuf::from)
            .unwrap_or_else(|| project.join(".plans"));
        (root.join(plan_arg), root)
    };
    if !valid_kebab(
        plan_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or_default(),
    ) {
        die("Plan directory name must be kebab-case", 64)
    }
    if plan_dir.exists() {
        eprintln!(
            "{COMMAND}: Plan directory already exists: {}",
            plan_dir.display()
        );
        std::process::exit(73)
    }
    if let Err(message) = require_safe_value("title", title) {
        die(message, 64)
    }
    fs::create_dir_all(&plans_root).unwrap_or_else(|error| die(error.to_string(), 64));
    let collisions = duplicate_steps(&plans_root);
    if !collisions.is_empty() {
        eprintln!("\n================================================================\nREFUSING TO CREATE A PLAN: a plan under this root has two steps\nsharing one number, so their execution order is undefined.\n================================================================\nPlans root: {}\nCollisions (goal, number, then the colliding files):\n  {}\n\nRename one of each pair to a free number, then create this plan.\nA step rename touches five surfaces: the step file, its testing\ncompanion, the inventory row File cell, the goal owned-unit blurb,\nand any progress tracker naming the step. Sweep all five.\nplan-content.sh find <plan> <step-name> --in all lists them.\n", plans_root.display(), collisions.join("\n  "));
        std::process::exit(73)
    }
    fs::create_dir(&plan_dir).unwrap_or_else(|error| die(error.to_string(), 64));
    let description = format!("# Plan: {title}\n\n## Current state\n\n§ 2.1\n<confirmed facts, available assets, and relevant prior work>\n\n## Desired outcome\n\n§ 3.1\n<definition of done>\n\n## Approach\n\n§ 4.1\n<agreed sequence and major implementation decisions>\n\n## Scope\n\n§ 5.1\n<included and explicitly excluded behavior>\n\n## Affected areas\n\n§ 6.1\n<files, modules, layouts, services, data, and systems>\n\n## Constraints and decisions\n\n§ 7.1\n<permissions, ownership, conventions, and user choices>\n\n## Risks and open questions\n\n§ 8.1\n<items that could affect execution>\n\n## Environment facts\n\n§ 9.1\n<host or URL to verify on, auth route, and the order in which steps verify against the running application>\n\n## Approach decisions\n\n§ 10.1\n<mechanism choices as prose: where each change lives and why, and alternatives considered and rejected>\n\n## Assumptions\n\n§ 11.1\n<what was assumed rather than confirmed, and what would change if it is wrong>\n\n## UI classification\n\n- UI affected: no\n- Rationale: <why>\n\n## Adversarial review\n\n- Artifact: `adversarial-review.md`\n- Status: 💤 pending\n");
    write_text(&plan_dir.join("plan-description.md"), &description);
    write_text(&plan_dir.join("commands.json"), "{}\n");
    let inventory = format!("# Work-unit inventory: {}\n\n## Definition-of-done coverage\n\n| Required outcome or proof | Work unit IDs | Notes |\n|---|---|---|\n\n## Work units\n\n| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |\n|---|---|---|---|---|---|---|---|---|\n\n## Decomposition review\n\n- [ ] Every definition-of-done item maps to one or more work units.\n- [ ] Every known affected file and changing symbol has its own work unit.\n- [ ] Every work unit has exactly one goal and one step.\n- [ ] Each goal has 2–10 work units, or records an allowed exception.\n- [ ] Each step has exactly one work unit and no unnamed incidental edits.\n- [ ] Dependencies form an executable order with no cycle.\n", plan_dir.file_name().unwrap().to_string_lossy());
    write_text(&plan_dir.join("work-unit-inventory.md"), &inventory);
    let plan_root = fs::canonicalize(&plan_dir).unwrap_or_else(|error| die(error.to_string(), 64));
    let root = fs::canonicalize(&plans_root).unwrap_or_else(|error| die(error.to_string(), 64));
    let skill = fs::canonicalize(&planning_root).unwrap_or_else(|error| die(error.to_string(), 64));
    let snapshot = match git_value(&plan_root, &["rev-parse", "--show-toplevel"]) {
        Some(top) => {
            let top = PathBuf::from(top);
            if git_ignored(&top, &plan_root) && top == root {
                root.display().to_string()
            } else {
                String::new()
            }
        }
        None if bare_name => root.display().to_string(),
        None => plan_root.display().to_string(),
    };
    write_env_manifest(
        &root.join(".env"),
        &[
            ("PLAN_ENV_SCHEMA_VERSION", "2".into()),
            ("PLANS_ROOT", root.display().to_string()),
            ("PLANNING_SKILL_ROOT", skill.display().to_string()),
            (
                "PLANNING_SCRIPTS_ROOT",
                skill.join("scripts").display().to_string(),
            ),
            (
                "PLANNING_TESTS_ROOT",
                skill.join("tests").display().to_string(),
            ),
        ],
    )
    .unwrap_or_else(|error| die(error, 66));
    write_env_manifest(
        &plan_root.join(".env"),
        &[
            ("PLAN_ENV_SCHEMA_VERSION", "2".into()),
            ("PLAN_SNAPSHOT_REPO", snapshot),
            ("PLANS_ROOT", root.display().to_string()),
            ("PLAN_ROOT", plan_root.display().to_string()),
            (
                "PLAN_NAME",
                plan_root.file_name().unwrap().to_string_lossy().into(),
            ),
            (
                "GLOBAL_PLANS_ENV_FILE",
                root.join(".env").display().to_string(),
            ),
            (
                "PLAN_ENV_FILE",
                plan_root.join(".env").display().to_string(),
            ),
            (
                "PLAN_DESCRIPTION_FILE",
                plan_root.join("plan-description.md").display().to_string(),
            ),
            (
                "PLAN_PROGRESS_FILE",
                plan_root.join("progress.md").display().to_string(),
            ),
            (
                "PLAN_WORK_UNIT_INVENTORY",
                plan_root
                    .join("work-unit-inventory.md")
                    .display()
                    .to_string(),
            ),
            (
                "PLAN_VALIDATION_FILE",
                plan_root.join("validation-report.md").display().to_string(),
            ),
            (
                "PLAN_CONTEXT_ROOT",
                plan_root.join("context").display().to_string(),
            ),
            (
                "PLAN_STEPS_ROOT",
                plan_root.join("steps").display().to_string(),
            ),
        ],
    )
    .unwrap_or_else(|error| die(error, 66));
    initialise_git(&plan_root, &root, bare_name);
    println!("Created {}", plan_dir.display());
    if let Some(commit) = git_value(&skill, &["rev-parse", "--short", "HEAD"]) {
        println!("planning skill: checkout at {commit}");
    }
}
