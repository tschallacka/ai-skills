// MODE: DEV
// PACKAGE: PROD
//! Moves plans out of the old per-agent `<target>/planning/plans` directories
//! into the single portable plan root -- ported from
//! installer/src/65-plan-migration.sh's `legacy_plan_migration`, and from
//! `plan_default_root`/`plan_ensure_root_permissions` in
//! planning/scripts/plan-core-lib.sh (the two calls this step makes into
//! the planning skill's own library, reimplemented here since this
//! installer does not source bash).
//!
//! Each plan is keyed by a hash of its own source path (blake3 instead of
//! bash's `cksum`, this installer's own manifest convention, same choice as
//! digest.rs), so a run that dies partway is idempotent: a plan already
//! marked `.complete` is skipped, and one still `.moving` or `.blocked` is
//! retried. Unlike install.sh's `mv` (which coreutils falls back to a
//! copy-then-remove for automatically), this uses `fs::rename` directly and
//! reports a cross-filesystem failure as blocked rather than silently
//! copying -- a gap worth knowing about if the plans root ever lives on a
//! different filesystem than a target root, which it does not for any
//! shipped target today (both sit under $HOME).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub fn default_root(home: &Path) -> PathBuf {
    if let Ok(root) = std::env::var("PLANS_ROOT") {
        if !root.is_empty() {
            return PathBuf::from(root.trim_end_matches('/').to_string());
        }
    }
    let base = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"));
    base.join("tsch-ai-skills").join("plans")
}

/// Creates the plan root if missing and proves it is actually writable with
/// a real probe file, same as install.sh refusing to trust `-w` alone.
pub fn ensure_root(root: &Path) -> io::Result<()> {
    fs::create_dir_all(root)?;
    let probe = root.join(format!(".permission-probe.{}", std::process::id()));
    fs::write(&probe, "")?;
    fs::remove_file(&probe)
}

pub struct MigrationOutcome {
    pub plan_root: PathBuf,
    pub migrated: Vec<PathBuf>,
    pub blocked: Vec<(PathBuf, String)>,
}

fn marker_path(state_dir: &Path, plan: &Path) -> PathBuf {
    let key = blake3::hash(plan.to_string_lossy().as_bytes()).to_hex();
    state_dir.join(key.to_string())
}

fn migrate_one(
    state_dir: &Path,
    plan: &Path,
    plan_root: &Path,
) -> io::Result<Result<PathBuf, String>> {
    let marker = marker_path(state_dir, plan);
    if marker.with_extension("complete").is_file() {
        return Ok(Err("already migrated".to_string()));
    }
    let Some(name) = plan.file_name() else {
        return Ok(Err("plan path has no file name".to_string()));
    };
    let destination = plan_root.join(name);
    if destination.exists() || destination.symlink_metadata().is_ok() {
        fs::write(
            marker.with_extension("blocked"),
            plan.to_string_lossy().as_bytes(),
        )?;
        return Ok(Err(format!(
            "collision, human review required: {} -> {}",
            plan.display(),
            destination.display()
        )));
    }
    fs::write(
        marker.with_extension("moving"),
        plan.to_string_lossy().as_bytes(),
    )?;
    match fs::rename(plan, &destination) {
        Ok(()) => {
            let _ = fs::remove_file(marker.with_extension("moving"));
            fs::write(
                marker.with_extension("complete"),
                plan.to_string_lossy().as_bytes(),
            )?;
            Ok(Ok(destination))
        }
        Err(e) => {
            fs::write(
                marker.with_extension("blocked"),
                plan.to_string_lossy().as_bytes(),
            )?;
            Ok(Err(format!("rerun after fixing permissions: {e}")))
        }
    }
}

/// Runs after an install loop, for every selected target root: any plan
/// still sitting under that root's own `planning/plans` moves to the shared
/// portable root. Not sorted for any semantic reason (each plan is
/// independent and keyed by its own path), just for reproducible test output.
pub fn migrate_legacy_plans(target_roots: &[PathBuf], home: &Path) -> io::Result<MigrationOutcome> {
    let plan_root = default_root(home);
    ensure_root(&plan_root)?;
    let state_dir = plan_root.join(".migration-state");
    fs::create_dir_all(&state_dir)?;

    let mut migrated = Vec::new();
    let mut blocked = Vec::new();
    for root in target_roots {
        let source_dir = root.join("planning").join("plans");
        if !source_dir.is_dir() {
            continue;
        }
        let mut plans: Vec<PathBuf> = fs::read_dir(&source_dir)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .collect();
        plans.sort();
        for plan in plans {
            match migrate_one(&state_dir, &plan, &plan_root)? {
                Ok(destination) => migrated.push(destination),
                Err(reason) if reason == "already migrated" => {}
                Err(reason) => blocked.push((plan, reason)),
            }
        }
    }
    Ok(MigrationOutcome {
        plan_root,
        migrated,
        blocked,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // default_root reads process-global env vars (PLANS_ROOT,
    // XDG_CONFIG_HOME); every test in this module that sets or clears them
    // takes this lock first so cargo's parallel test threads cannot
    // interleave one test's env mutation with another's read -- exactly the
    // kind of shell-level flakiness this installer rewrite exists to avoid.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn plan_dir(root: &Path, target: &str, plan_name: &str) -> PathBuf {
        let dir = root
            .join(target)
            .join("planning")
            .join("plans")
            .join(plan_name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("plan-description.md"), "# Plan: x\n").unwrap();
        dir
    }

    #[test]
    fn default_root_honors_plans_root_override() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::set_var("PLANS_ROOT", "/custom/root/");
        assert_eq!(
            default_root(Path::new("/home/x")),
            PathBuf::from("/custom/root")
        );
        std::env::remove_var("PLANS_ROOT");
    }

    #[test]
    fn default_root_falls_back_to_home_config() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("PLANS_ROOT");
        std::env::remove_var("XDG_CONFIG_HOME");
        assert_eq!(
            default_root(Path::new("/home/x")),
            PathBuf::from("/home/x/.config/tsch-ai-skills/plans")
        );
    }

    #[test]
    fn a_plan_under_a_targets_planning_directory_is_moved_to_the_shared_root() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("PLANS_ROOT");
        let workdir = tempfile::tempdir().unwrap();
        let home = workdir.path().join("home");
        let targets_root = workdir.path().join("targets");
        plan_dir(&targets_root, "claude", "my-plan");

        let outcome = migrate_legacy_plans(&[targets_root.join("claude")], &home).unwrap();

        assert_eq!(outcome.migrated.len(), 1);
        assert!(outcome.blocked.is_empty());
        assert!(outcome
            .plan_root
            .join("my-plan/plan-description.md")
            .is_file());
        assert!(!targets_root.join("claude/planning/plans/my-plan").exists());
    }

    #[test]
    fn a_rerun_after_a_successful_migration_does_nothing_more() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("PLANS_ROOT");
        let workdir = tempfile::tempdir().unwrap();
        let home = workdir.path().join("home");
        let targets_root = workdir.path().join("targets");
        plan_dir(&targets_root, "claude", "my-plan");
        migrate_legacy_plans(&[targets_root.join("claude")], &home).unwrap();

        // The source is gone, so a second run has nothing to look at -- but
        // even if a partial retry recreated the source dir, the .complete
        // marker keyed on that exact path would still skip it.
        let outcome = migrate_legacy_plans(&[targets_root.join("claude")], &home).unwrap();
        assert!(outcome.migrated.is_empty());
        assert!(outcome.blocked.is_empty());
    }

    #[test]
    fn a_name_collision_at_the_destination_is_reported_and_left_in_place() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("PLANS_ROOT");
        let workdir = tempfile::tempdir().unwrap();
        let home = workdir.path().join("home");
        let targets_root = workdir.path().join("targets");
        let plan = plan_dir(&targets_root, "claude", "dup-plan");

        let plan_root = default_root(&home);
        fs::create_dir_all(plan_root.join("dup-plan")).unwrap();

        let outcome = migrate_legacy_plans(&[targets_root.join("claude")], &home).unwrap();
        assert!(outcome.migrated.is_empty());
        assert_eq!(outcome.blocked.len(), 1);
        assert!(outcome.blocked[0].1.contains("collision"));
        assert!(
            plan.is_dir(),
            "the source plan must stay put on a collision"
        );
    }

    #[test]
    fn two_targets_each_contribute_their_own_plans() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("PLANS_ROOT");
        let workdir = tempfile::tempdir().unwrap();
        let home = workdir.path().join("home");
        let targets_root = workdir.path().join("targets");
        plan_dir(&targets_root, "claude", "plan-a");
        plan_dir(&targets_root, "opencode", "plan-b");

        let outcome = migrate_legacy_plans(
            &[targets_root.join("claude"), targets_root.join("opencode")],
            &home,
        )
        .unwrap();

        assert_eq!(outcome.migrated.len(), 2);
        assert!(outcome.plan_root.join("plan-a").is_dir());
        assert!(outcome.plan_root.join("plan-b").is_dir());
    }

    #[test]
    fn a_target_with_no_planning_directory_is_skipped_without_error() {
        let _guard = ENV_LOCK.lock().unwrap();
        std::env::remove_var("PLANS_ROOT");
        let workdir = tempfile::tempdir().unwrap();
        let home = workdir.path().join("home");
        let outcome =
            migrate_legacy_plans(&[workdir.path().join("no-such-target")], &home).unwrap();
        assert!(outcome.migrated.is_empty());
        assert!(outcome.blocked.is_empty());
    }
}
