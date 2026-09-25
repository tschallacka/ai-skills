// MODE: DEV
// PACKAGE: PROD
use planning_core::{project_root_for, require_safe_value, write_env_manifest};
use std::collections::BTreeMap;
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

fn git_succeeds(directory: &Path, args: &[&str]) -> bool {
    Command::new("git")
        .arg("-C")
        .arg(directory)
        .args(args)
        .output()
        .is_ok_and(|output| output.status.success())
}

/// The planning skill directory this run belongs to. The wrapper exports
/// PLANNING_SKILL_ROOT (the directory that CONTAINS `planning/scripts`), which
/// is what makes an installed copy report itself rather than the tree this
/// binary happened to be compiled in; the compile-time path is the fallback for
/// a binary run directly.
fn skill_dir() -> PathBuf {
    if let Some(root) = env::var_os("PLANNING_SKILL_ROOT").filter(|value| !value.is_empty()) {
        let dir = PathBuf::from(root).join("planning");
        if dir.join("scripts").is_dir() {
            return dir;
        }
    }
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../planning")
}

/// The commit an installer recorded in `.version` (`source_version=branch:x
/// commit:abc123`): the last `commit:` on the first line carrying one.
fn version_commit(marker: &str) -> String {
    marker
        .lines()
        .find_map(|line| {
            line.rfind("commit:")
                .map(|at| &line[at + "commit:".len()..])
        })
        .map(|rest| {
            rest.chars()
                .take_while(|character| matches!(character, '0'..='9' | 'a'..='f'))
                .collect()
        })
        .unwrap_or_default()
}

/// One line naming which build of the planning skill is running. A checkout
/// reports its commit, which is exact; an installed copy reports what the
/// installer recorded at install time, worded so it is not mistaken for a
/// statement about what the canonical checkout holds now (T52).
fn skill_provenance(skill: &Path) -> Option<String> {
    if let Some(commit) = git_value(skill, &["rev-parse", "--short", "HEAD"]) {
        return Some(format!("planning skill: checkout at {commit}"));
    }
    let marker = fs::read_to_string(skill.join(".version")).ok()?;
    let package = marker
        .lines()
        .find_map(|line| line.strip_prefix("package_version="))
        .unwrap_or("");
    let commit = version_commit(&marker);
    if package.is_empty() && commit.is_empty() {
        return None;
    }
    let or_unknown = |value: &str| if value.is_empty() { "unknown" } else { value }.to_string();
    Some(format!(
        "planning skill: installed build {}, from commit {}",
        or_unknown(package),
        or_unknown(&commit)
    ))
}

/// Warn on stderr when the running skill is an installed copy built from a
/// commit that a reachable canonical checkout (AI_SKILLS_REPO) does not contain
/// or has moved past. Silent when it cannot tell, which is most of the time:
/// without a named checkout there is nothing to compare against, and a wrong
/// staleness warning sends someone to reinstall for nothing.
fn warn_skill_drift(skill: &Path) {
    let Some(repo) = env::var_os("AI_SKILLS_REPO")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
    else {
        return;
    };
    if !repo.join(".git").exists() || git_succeeds(skill, &["rev-parse", "--git-dir"]) {
        return;
    }
    let commit = fs::read_to_string(skill.join(".version"))
        .map(|marker| version_commit(&marker))
        .unwrap_or_default();
    if commit.is_empty() {
        return;
    }
    if !git_succeeds(&repo, &["cat-file", "-e", &format!("{commit}^{{commit}}")]) {
        eprintln!(
            "planning: this skill was installed from commit {commit}, which {} does not contain; reinstall before trusting it",
            repo.display()
        );
        return;
    }
    if !git_succeeds(&repo, &["merge-base", "--is-ancestor", &commit, "HEAD"]) {
        return;
    }
    let behind = git_value(&repo, &["rev-list", "--count", &format!("{commit}..HEAD")])
        .and_then(|count| count.parse::<u64>().ok())
        .unwrap_or(0);
    if behind > 0 {
        eprintln!(
            "planning: this skill was installed from {commit}, {behind} commit(s) behind {}",
            repo.display()
        );
    }
}

fn duplicate_steps(root: &Path) -> Vec<String> {
    let mut collisions = Vec::new();
    let Ok(plans) = fs::read_dir(root) else {
        return collisions;
    };
    let mut plans: Vec<_> = plans
        .flatten()
        .filter(|entry| entry.path().is_dir())
        .collect();
    plans.sort_by_key(|entry| entry.file_name());
    for plan in plans {
        let plan_name = plan.file_name().to_string_lossy().into_owned();
        let Ok(goals) = fs::read_dir(plan.path()) else {
            continue;
        };
        let mut goals: Vec<_> = goals
            .flatten()
            .filter(|entry| entry.path().is_dir())
            .collect();
        goals.sort_by_key(|entry| entry.file_name());
        for goal in goals {
            let steps = goal.path().join("steps");
            let Ok(entries) = fs::read_dir(&steps) else {
                continue;
            };
            let mut by_number: BTreeMap<String, Vec<String>> = BTreeMap::new();
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
                by_number.entry(number.to_string()).or_default().push(file);
            }
            // One line per collision, naming the files that share the number:
            // the caller has to rename one of them, so the report is useless
            // without them.
            for (number, mut files) in by_number {
                if files.len() > 1 {
                    files.sort();
                    collisions.push(format!(
                        "{plan_name}: goal {} {number} {}",
                        goal.file_name().to_string_lossy(),
                        files.join(" ")
                    ));
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

/// Git runs its automatic maintenance DETACHED after a commit, so it outlives
/// the tool that committed: it takes `.git/objects/maintenance.lock` (and, when
/// it repacks, temporary pack and bitmap files) in and out under whatever reads,
/// copies or removes the plan next. Foreground keeps the maintenance and drops
/// the stray process.
///
/// Only for a repository this tool has just created: an existing project's own
/// repository keeps whatever the user configured.
fn keep_maintenance_in_the_foreground(repo: &Path) {
    for key in ["gc.autoDetach", "maintenance.autoDetach"] {
        git_succeeds(repo, &["config", key, "false"]);
    }
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
    let repo = repo.unwrap_or_else(|| {
        if bare_name {
            plans_root.to_path_buf()
        } else {
            plan.to_path_buf()
        }
    });
    // Always `git init` the chosen repository: a git-ignored plan under a
    // project's work tree is committed into its OWN repository at the plans
    // root, and without this the `add` below runs against the enclosing
    // project's repository, refuses the ignored path, and leaves the plan with
    // no history for any later pre-mutation snapshot. Re-initialising an
    // existing repository is a no-op.
    let _ = fs::create_dir_all(&repo);
    let created = !repo.join(".git").exists();
    let _ = Command::new("git").args(["init", "-q"]).arg(&repo).status();
    if created {
        keep_maintenance_in_the_foreground(&repo);
    }
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
    // A bare name has no path separator: `/` everywhere, and the platform's own
    // (`\` on Windows) as well.
    let names_a_path = plan_arg.contains(['/', std::path::MAIN_SEPARATOR]);
    let bare_name = !names_a_path;
    let planning_root = skill_dir();
    let (plan_dir, plans_root) = if names_a_path {
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
    // planning_core::canonicalize, not fs::canonicalize: on Windows the latter
    // yields `\\?\C:\...`, which the manifests written below are sourced by
    // bash and read by git, and neither can open.
    let plan_root =
        planning_core::canonicalize(&plan_dir).unwrap_or_else(|error| die(error.to_string(), 64));
    let root =
        planning_core::canonicalize(&plans_root).unwrap_or_else(|error| die(error.to_string(), 64));
    let skill = planning_core::canonicalize(&planning_root)
        .unwrap_or_else(|error| die(error.to_string(), 64));
    let snapshot = match git_value(&plan_root, &["rev-parse", "--show-toplevel"]) {
        Some(top) => {
            // The repository that will hold this plan's history, decided the
            // way `initialise_git` decides it: a plan git-ignores under some
            // enclosing work tree (a project's `.plans`) gets its own
            // repository at the plans root, so that repository is what is
            // snapshotted; a plan tracked in the user's tree is theirs, and
            // stays unpinned.
            let top = PathBuf::from(top);
            let repo = if git_ignored(&top, &plan_root) {
                root.clone()
            } else {
                top
            };
            if repo == root {
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
                // file_name() is None if plan_root resolves to the
                // filesystem root; plan_root is already canonicalized above,
                // but this stays defensive rather than assuming that can
                // never happen (B338).
                plan_root
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| ".".to_string()),
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
    if let Some(line) = skill_provenance(&skill) {
        println!("{line}");
    }
    warn_skill_drift(&skill);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_commit_reads_the_last_commit_marker_on_the_line() {
        assert_eq!(
            version_commit("package_version=1\nsource_version=branch:x commit:4a5f1a35\n"),
            "4a5f1a35"
        );
        assert_eq!(version_commit("source_version=branch:x\n"), "");
    }

    #[test]
    fn an_installed_copy_reports_its_recorded_build_and_never_claims_a_checkout() {
        let dir = env::temp_dir().join(format!("create-plan-provenance-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join(".version"),
            "package_version=9.9.9\nsource_version=branch:x commit:deadbee\n",
        )
        .unwrap();
        // Skip the assertion if the scratch directory sits inside a git tree:
        // that is the "checkout" shape by definition.
        if git_value(&dir, &["rev-parse", "--short", "HEAD"]).is_none() {
            assert_eq!(
                skill_provenance(&dir).as_deref(),
                Some("planning skill: installed build 9.9.9, from commit deadbee")
            );
        }
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_duplicate_step_number_is_reported_with_the_files_that_share_it() {
        let root = env::temp_dir().join(format!("create-plan-dup-{}", std::process::id()));
        let steps = root.join("broken/02-research/steps");
        fs::create_dir_all(&steps).unwrap();
        for name in [
            "01-step-a.md",
            "01-step-collision.md",
            "01-step-a-testing.md",
            "02-step-b.md",
        ] {
            fs::write(steps.join(name), "x\n").unwrap();
        }
        assert_eq!(
            duplicate_steps(&root),
            vec!["broken: goal 02-research 01 01-step-a.md 01-step-collision.md".to_string()]
        );
        let _ = fs::remove_dir_all(&root);
    }
}
