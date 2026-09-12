// MODE: DEV
// PACKAGE: PROD
//! Machine-facing CLI-mode subcommands install.sh exposes for the planning
//! skill's own self-update tooling -- ported from install.sh's
//! `--print-skill-files`/`--resolve-source`/`--install-skill` and their
//! handlers (`cli_print_skill_files`/`cli_resolve_source`/`cli_install_skill`,
//! installer/src/55-cli-handlers.sh).
//!
//! Deliberately distinct from the normal install path (install.rs) rather
//! than routing through it: install.sh's own CLI mode refuses the whole
//! install on any unmanaged collision instead of backing up and
//! overwriting, and tracks a single `.version` marker rather than
//! install.rs's per-file `.filehashes` digest -- this ports that same,
//! stricter contract as its own thing, matching what bash itself does (its
//! CLI handlers are a separate code path from the interactive install.sh
//! function too).

use crate::install;
use crate::integration;
use crate::manifest;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn git_output(source_root: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new("git")
        .arg("-C")
        .arg(source_root)
        .args(args)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8(output.stdout).ok()?;
    let text = text.trim();
    if text.is_empty() {
        None
    } else {
        Some(text.to_string())
    }
}

/// This binary's own answer to install.sh's `SOURCE_VERSION` -- ported from
/// `download_source`'s local-checkout branch only. Unlike install.sh, this
/// binary never downloads a source tree itself (`installer/bootstrap.sh`
/// does that before this binary ever runs), so `download_source`'s curl/
/// tarball branches (a tag, a bare commit, or a remote branch ref) have
/// nothing to port here -- `--source` always names a directory already on
/// disk, which is exactly bash's "found `planning/SKILL.md` next to
/// `$BASH_SOURCE`" case.
fn source_version(source_root: &Path) -> String {
    let commit = git_output(source_root, &["rev-parse", "--short=12", "HEAD"])
        .unwrap_or_else(|| "unknown".to_string());
    match git_output(
        source_root,
        &["describe", "--tags", "--exact-match", "HEAD"],
    ) {
        Some(tag) => format!("tag:{tag} commit:{commit}"),
        None => {
            let branch = git_output(source_root, &["symbolic-ref", "--short", "-q", "HEAD"])
                .unwrap_or_else(|| "detached".to_string());
            format!("branch:{branch} commit:{commit}")
        }
    }
}

/// `"version"` from `package.json`'s top level, the same sed-extractable
/// shape install.sh's own version_marker_content reads (rjq is a declared
/// runtime dependency of some skills, so this file has to be readable
/// before any skill's own tools are known to exist).
fn package_version(source_root: &Path) -> String {
    let Ok(content) = fs::read_to_string(source_root.join("package.json")) else {
        return "unknown".to_string();
    };
    for line in content.lines() {
        let trimmed = line.trim();
        let Some(rest) = trimmed.strip_prefix("\"version\"") else {
            continue;
        };
        let Some(colon) = rest.find(':') else {
            continue;
        };
        let value = rest[colon + 1..].trim();
        let Some(value) = value.strip_prefix('"') else {
            continue;
        };
        if let Some(end) = value.find('"') {
            return value[..end].to_string();
        }
    }
    "unknown".to_string()
}

/// installer/src/50-manifest.sh's `version_marker_content`. `AI_SKILLS_REF`
/// mirrors install.sh's own env var of the same name, defaulting to
/// `master`.
pub fn version_marker_content(source_root: &Path) -> String {
    let repo_ref = std::env::var("AI_SKILLS_REF").unwrap_or_else(|_| "master".to_string());
    format!(
        "format=ai-skills-version-1\npackage_version={}\nsource_version={}\nsource_ref={repo_ref}\n",
        package_version(source_root),
        source_version(source_root),
    )
}

/// `installer print-skill-files planning` -- ported from
/// `cli_print_skill_files`. Refuses any skill but planning, same as bash:
/// this is planning's own self-update tooling, not a general-purpose file
/// lister.
pub fn print_skill_files(source_root: &Path, skill: &str) -> Result<String, String> {
    if skill != "planning" {
        return Err(format!("unsupported CLI skill: {skill}"));
    }
    fs::read_to_string(source_root.join("planning/PACKAGE-MANIFEST.tsv")).map_err(|e| e.to_string())
}

/// `installer resolve-source planning <relative>` -- ported from
/// `cli_resolve_source`.
pub fn resolve_source_file(
    source_root: &Path,
    skill: &str,
    relative: &str,
) -> Result<PathBuf, String> {
    if skill != "planning" {
        return Err(format!("unsupported CLI skill: {skill}"));
    }
    let source = source_root.join(skill).join(relative);
    if !source.is_file() {
        return Err(format!("source does not exist: {relative}"));
    }
    Ok(source)
}

#[derive(Debug)]
pub enum CliInstallOutcome {
    Installed(PathBuf),
    ApprovalDeclined,
    Collision,
}

/// `installer install-skill <skill> --target DIR --approval yes|no` --
/// ported from `cli_install_skill`. Unlike install.rs's own `install_skill`
/// (which backs up a changed file and overwrites it unconditionally), this
/// refuses the whole install on ANY unmanaged collision -- an existing file
/// or symlink this run does not already own -- unless every collision is
/// exactly the shared `.version` marker showing this is an upgrade of an
/// install this same mechanism made (`managed_version_transition`), and
/// even then only when nothing collided is itself a symlink. Exit-code
/// contract, mapped by the caller: 0 installed, 2 approval declined
/// (nothing written), 3 an unsafe or unmanaged collision (nothing written)
/// -- install.sh's own documented contract for this entry point.
pub fn install_skill_cli(
    source_root: &Path,
    skill: &str,
    target: &Path,
    approval_yes: bool,
    package_dev: bool,
) -> Result<CliInstallOutcome, String> {
    if manifest::known_skill(skill).is_none() {
        return Err(format!("unsupported CLI skill: {skill}"));
    }
    if let Some(reason) = manifest::skill_unsupported_here(skill) {
        return Err(format!("{skill} cannot be installed here: {reason}"));
    }
    let status = crate::requirements::skill_status(source_root, skill);
    if status.state == crate::requirements::SkillState::Blocked {
        return Err(format!(
            "{skill} is missing a hard runtime requirement; nothing was written"
        ));
    }

    let dest_dir = target.join(skill);
    let mode = integration::resolve_mode(source_root, skill, Some(&dest_dir), None);
    let relative_paths = install::skill_relative_files(source_root, skill, package_dev)
        .map_err(|e| e.to_string())?;

    let mut collision = false;
    let mut unsafe_collision = false;
    for relative in &relative_paths {
        if !integration::file_allowed(source_root, skill, relative, &mode) {
            continue;
        }
        let source_file = source_root.join(skill).join(relative);
        if !source_file.is_file() {
            return Err(format!("source does not exist: {relative}"));
        }
        let dest_file = dest_dir.join(relative);
        if dest_file.exists() || dest_file.is_symlink() {
            eprintln!("Collision: {}", dest_file.display());
            collision = true;
            if dest_file.is_symlink() {
                unsafe_collision = true;
            }
        }
    }

    let version_path = dest_dir.join(".version");
    let mut managed_version_transition = false;
    if version_path.exists() || version_path.is_symlink() {
        eprintln!("Collision: {}", version_path.display());
        collision = true;
        if version_path.is_symlink() {
            unsafe_collision = true;
        }
        if version_path.is_file() {
            let current = version_marker_content(source_root);
            let existing = fs::read_to_string(&version_path).unwrap_or_default();
            if existing != current {
                managed_version_transition = true;
            }
        }
    }

    if collision && (!managed_version_transition || unsafe_collision) {
        return Ok(CliInstallOutcome::Collision);
    }
    if !approval_yes {
        eprintln!("Approval declined; no files changed.");
        return Ok(CliInstallOutcome::ApprovalDeclined);
    }

    for relative in &relative_paths {
        if !integration::file_allowed(source_root, skill, relative, &mode) {
            continue;
        }
        let source_file = source_root.join(skill).join(relative);
        let dest_file = dest_dir.join(relative);
        if let Some(parent) = dest_file.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::copy(&source_file, &dest_file).map_err(|e| e.to_string())?;
    }
    fs::write(&version_path, version_marker_content(source_root)).map_err(|e| e.to_string())?;
    Ok(CliInstallOutcome::Installed(dest_dir))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;

    fn write(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        let mut f = fs::File::create(path).unwrap();
        f.write_all(content.as_bytes()).unwrap();
    }

    #[test]
    fn print_skill_files_refuses_any_skill_but_planning() {
        let dir = tempfile::tempdir().unwrap();
        let err = print_skill_files(dir.path(), "todo").unwrap_err();
        assert!(err.contains("unsupported CLI skill"));
    }

    #[test]
    fn print_skill_files_returns_the_planning_manifest() {
        let dir = tempfile::tempdir().unwrap();
        write(
            &dir.path().join("planning/PACKAGE-MANIFEST.tsv"),
            "relative\tpackage\nSKILL.md\tprod\n",
        );
        let content = print_skill_files(dir.path(), "planning").unwrap();
        assert!(content.contains("SKILL.md"));
    }

    #[test]
    fn resolve_source_file_refuses_any_skill_but_planning() {
        let dir = tempfile::tempdir().unwrap();
        let err = resolve_source_file(dir.path(), "todo", "SKILL.md").unwrap_err();
        assert!(err.contains("unsupported CLI skill"));
    }

    #[test]
    fn resolve_source_file_refuses_a_missing_relative_path() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("planning")).unwrap();
        let err = resolve_source_file(dir.path(), "planning", "nope.md").unwrap_err();
        assert!(err.contains("source does not exist"));
    }

    #[test]
    fn resolve_source_file_returns_the_absolute_path() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("planning/SKILL.md"), "# planning\n");
        let resolved = resolve_source_file(dir.path(), "planning", "SKILL.md").unwrap();
        assert_eq!(resolved, dir.path().join("planning/SKILL.md"));
    }

    #[test]
    fn install_skill_cli_refuses_an_unknown_skill() {
        let dir = tempfile::tempdir().unwrap();
        let target = tempfile::tempdir().unwrap();
        let err = install_skill_cli(dir.path(), "not-a-real-skill", target.path(), true, false)
            .unwrap_err();
        assert!(err.contains("unsupported CLI skill"));
    }

    #[test]
    fn install_skill_cli_installs_on_a_clean_target_with_approval() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("todo/SKILL.md"), "# todo\n");
        let target = tempfile::tempdir().unwrap();

        let outcome = install_skill_cli(dir.path(), "todo", target.path(), true, false).unwrap();
        assert!(matches!(outcome, CliInstallOutcome::Installed(_)));
        assert!(target.path().join("todo/SKILL.md").is_file());
        assert!(target.path().join("todo/.version").is_file());
    }

    #[test]
    fn install_skill_cli_declines_without_approval_and_writes_nothing() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("todo/SKILL.md"), "# todo\n");
        let target = tempfile::tempdir().unwrap();

        let outcome = install_skill_cli(dir.path(), "todo", target.path(), false, false).unwrap();
        assert!(matches!(outcome, CliInstallOutcome::ApprovalDeclined));
        assert!(!target.path().join("todo/SKILL.md").exists());
    }

    #[test]
    fn install_skill_cli_refuses_an_unmanaged_collision() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("todo/SKILL.md"), "# todo\n");
        let target = tempfile::tempdir().unwrap();
        write(
            &target.path().join("todo/SKILL.md"),
            "someone else's file\n",
        );

        let outcome = install_skill_cli(dir.path(), "todo", target.path(), true, false).unwrap();
        assert!(matches!(outcome, CliInstallOutcome::Collision));
        assert_eq!(
            fs::read_to_string(target.path().join("todo/SKILL.md")).unwrap(),
            "someone else's file\n"
        );
    }

    #[test]
    fn install_skill_cli_reinstalling_unchanged_content_is_still_a_collision() {
        // Bash's own rule: an identical .version marker means nothing
        // changed, which is NOT the upgrade case -- managed_version_transition
        // only fires when the marker DIFFERS. Re-running with nothing
        // changed at all is an ordinary, unmanaged-looking collision.
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("todo/SKILL.md"), "# todo v1\n");
        let target = tempfile::tempdir().unwrap();

        let first = install_skill_cli(dir.path(), "todo", target.path(), true, false).unwrap();
        assert!(matches!(first, CliInstallOutcome::Installed(_)));

        let second = install_skill_cli(dir.path(), "todo", target.path(), true, false).unwrap();
        assert!(matches!(second, CliInstallOutcome::Collision));
    }

    #[test]
    fn install_skill_cli_allows_a_managed_version_upgrade() {
        // A .version marker that already exists but DIFFERS from what this
        // run would write is exactly what a prior install by this same
        // mechanism, now upgrading, looks like -- allowed through even
        // though every managed file still collides.
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("todo/SKILL.md"), "# todo v2\n");
        let target = tempfile::tempdir().unwrap();
        write(&target.path().join("todo/SKILL.md"), "# todo v1 (old)\n");
        write(
            &target.path().join("todo/.version"),
            "format=ai-skills-version-1\nold marker\n",
        );

        let outcome = install_skill_cli(dir.path(), "todo", target.path(), true, false).unwrap();
        assert!(matches!(outcome, CliInstallOutcome::Installed(_)));
        assert_eq!(
            fs::read_to_string(target.path().join("todo/SKILL.md")).unwrap(),
            "# todo v2\n"
        );
    }

    #[test]
    fn install_skill_cli_refuses_even_a_managed_upgrade_over_a_symlink() {
        let dir = tempfile::tempdir().unwrap();
        write(&dir.path().join("todo/SKILL.md"), "# todo v2\n");
        let target = tempfile::tempdir().unwrap();
        fs::create_dir_all(target.path().join("todo")).unwrap();
        write(
            &target.path().join("todo/.version"),
            "format=ai-skills-version-1\nold marker\n",
        );
        let elsewhere = tempfile::tempdir().unwrap();
        write(&elsewhere.path().join("real-skill-md"), "not the real file");
        std::os::unix::fs::symlink(
            elsewhere.path().join("real-skill-md"),
            target.path().join("todo/SKILL.md"),
        )
        .unwrap();

        let outcome = install_skill_cli(dir.path(), "todo", target.path(), true, false).unwrap();
        assert!(matches!(outcome, CliInstallOutcome::Collision));
        assert!(target.path().join("todo/SKILL.md").is_symlink());
    }
}
