// MODE: DEV
// PACKAGE: PROD
//! T142 (first pass): remove a skill's own installed directory, garbage-
//! collect its shared binary (T72's shared_bin::shared_bin_dir) and any
//! co-installed vendor plugin only when nothing else still installed needs
//! them, and deregister its MCP entry -- reusing `mcp::unregister_for_kind`,
//! already built for a mode switch away from `mcp`.
//!
//! Deliberately NOT covered this pass, filed as a follow-up TODO instead:
//! permission-grant reversal (`permissions.rs` is add-only for claude/
//! opencode/codex settings -- no revoke exists for any of the three), the
//! opencode variant of tui-hint-plugin (shared by `$HOME` and registered by
//! path in one config file -- a different sharing model than the Claude
//! per-target-root copy this module's plugin GC handles), and a fine-grained
//! interactive TUI action (this ships CLI flags only, headless).
//!
//! A shared binary is keyed only by `$HOME` (shared_bin::shared_bin_dir),
//! not by target root, so removing it safely means checking every root this
//! installer can discover under that `$HOME` -- not just the one named in
//! this uninstall call -- or a sibling agent root could lose a binary it
//! still needs. `known_roots` is that discovery: every `manifest::AGENTS`
//! home directory, plus every path `custom_locations` has ever recorded.
//! Documented limitation, same class T72 already accepted for "no GC at
//! all": a `--target` path that was never a recognized agent root and never
//! saved as a custom location is invisible to this scan.

use crate::digest;
use crate::install;
use crate::integration;
use crate::manifest;
use crate::mcp;
use crate::permissions;
use crate::shared_bin;
use std::collections::HashSet;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// One vendor plugin's file-set name (matching `plugins.rs`'s own directory
/// names) and which skills, when any one of them is still installed on the
/// SAME target root, keep it from being removed -- the same companion
/// mapping `main.rs`'s post-install steps already hardcode for installing
/// these plugins in the first place.
const CLAUDE_PLUGINS: &[(&str, &[&str])] = &[
    ("tui-hint-plugin", &["interactive-shell"]),
    ("editor-gate-plugin", &["ai-text-editor"]),
    (
        "agent-identity-plugin",
        &["chat", "ai-text-editor", "interactive-shell"],
    ),
];

/// Every shared binary filename `skill`'s CURRENT install at `dest_dir`
/// needs, in its currently-resolved integration mode -- mirrors exactly the
/// shared-bin branch of `install::install_skill`'s own copy loop, so "what
/// does this skill need" is computed identically whether the skill is being
/// removed or is one of the "does anything else still need this" checks.
pub fn shared_binaries_needed_by(
    source_root: &Path,
    skill: &str,
    dest_dir: &Path,
) -> HashSet<String> {
    let mode = integration::resolve_mode(source_root, skill, Some(dest_dir), None);
    let Ok(relative_paths) = install::relative_paths_for(source_root, skill, true) else {
        return HashSet::new();
    };
    relative_paths
        .iter()
        .filter(|relative| integration::file_allowed(source_root, skill, relative, &mode))
        .filter_map(|relative| shared_bin::shared_binary_filename(relative))
        .map(str::to_string)
        .collect()
}

/// Every root this installer can discover under `home`: every known agent's
/// own skills directory, plus every custom location ever saved. See the
/// module doc comment for what this does not cover.
pub fn known_roots(home: &Path) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = manifest::AGENTS
        .iter()
        .map(|a| home.join(a.home_suffix))
        .collect();
    roots.extend(crate::custom_locations::load(home));
    roots
}

/// Subdirectories of `root` that are a currently-installed skill: their name
/// is a known skill and the directory exists on disk.
pub fn installed_skill_dirs(root: &Path) -> Vec<(String, PathBuf)> {
    manifest::SKILLS
        .iter()
        .map(|s| (s.name.to_string(), root.join(s.name)))
        .filter(|(_, dir)| dir.is_dir())
        .collect()
}

/// Does some OTHER still-installed skill, on any root this installer can
/// discover under `home`, still need `binary_filename`? `exclude_dir` is the
/// skill directory being removed -- it must not vote for its own binary's
/// survival, since by the time this is asked it is already gone (or about
/// to be).
pub fn other_skill_needs_binary(
    source_root: &Path,
    home: &Path,
    binary_filename: &str,
    exclude_dir: &Path,
) -> bool {
    for root in known_roots(home) {
        for (skill, dir) in installed_skill_dirs(&root) {
            if dir == exclude_dir {
                continue;
            }
            if shared_binaries_needed_by(source_root, &skill, &dir).contains(binary_filename) {
                return true;
            }
        }
    }
    false
}

/// Does some OTHER still-installed skill on `target_root` (the same root
/// the plugin was copied into) still need the plugin named `plugin_name`?
/// Unlike a shared binary, a vendor plugin is copied per target root, not
/// shared by `$HOME`, so this checks only that one root.
fn other_skill_on_root_needs_plugin(
    target_root: &Path,
    plugin_name: &str,
    exclude_skill: &str,
) -> bool {
    let Some((_, companions)) = CLAUDE_PLUGINS.iter().find(|(name, _)| *name == plugin_name) else {
        return false;
    };
    companions
        .iter()
        .any(|companion| *companion != exclude_skill && target_root.join(companion).is_dir())
}

pub struct UninstallReport {
    pub was_installed: bool,
    pub removed_shared_binaries: Vec<String>,
    pub kept_shared_binaries: Vec<String>,
    pub removed_plugins: Vec<String>,
    pub mcp_entry_removed: bool,
    pub modified_files: Vec<String>,
    pub permissions_removed: Vec<String>,
    pub opencode_tui_hint_plugin_removed: bool,
}

/// Reverses `skill`'s own per-skill permission grant on `target_root`, if it
/// has one. Only interactive-shell's Bash-execute grant is per-skill today;
/// the planning and worktree grants are run-wide (added once for the whole
/// run, not tied to any one skill's install), so they are deliberately left
/// untouched here and remain a future whole-run "deprovision" command's job.
fn revoke_skill_permissions(
    skill: &str,
    target_root: &Path,
    kind: &str,
    home: &Path,
) -> Vec<String> {
    if skill != "interactive-shell" {
        return Vec::new();
    }
    let bins = target_root.join("interactive-shell").join("bin");
    let bins = bins.to_string_lossy();
    let removed = match kind {
        "claude" => permissions::claude_interactive_shell_permissions_remove(&bins, home).ok(),
        _ => None,
    };
    match removed {
        Some(permissions::PermissionRemovalOutcome::Removed(entries)) => entries,
        _ => Vec::new(),
    }
}

pub struct UninstallPreview {
    pub would_remove_shared_binaries: Vec<String>,
    pub would_keep_shared_binaries: Vec<String>,
    pub would_remove_plugins: Vec<String>,
    pub has_mcp_entry: bool,
    pub modified_files: Vec<String>,
}

/// The read-only counterpart to `uninstall_skill`: computes what it WOULD
/// do without deleting or writing anything, so a UI can show a preview
/// before the user confirms. Reuses the same "does anything else still
/// need this" helpers `uninstall_skill` itself calls; permission-grant and
/// MCP-registration removal are not previewed in detail here (both are
/// reversible config edits, not data loss, so the skill/binary/plugin
/// preview is what actually matters before confirming).
pub fn preview_uninstall(
    source_root: &Path,
    skill: &str,
    target_root: &Path,
    home: &Path,
    kind: Option<&str>,
) -> UninstallPreview {
    let dest_dir = target_root.join(skill);
    if !dest_dir.is_dir() {
        return UninstallPreview {
            would_remove_shared_binaries: Vec::new(),
            would_keep_shared_binaries: Vec::new(),
            would_remove_plugins: Vec::new(),
            has_mcp_entry: false,
            modified_files: Vec::new(),
        };
    }

    let needed = shared_binaries_needed_by(source_root, skill, &dest_dir);
    let mut would_remove_shared_binaries = Vec::new();
    let mut would_keep_shared_binaries = Vec::new();
    for filename in &needed {
        if other_skill_needs_binary(source_root, home, filename, &dest_dir) {
            would_keep_shared_binaries.push(filename.clone());
        } else {
            would_remove_shared_binaries.push(filename.clone());
        }
    }
    would_remove_shared_binaries.sort();
    would_keep_shared_binaries.sort();

    let mut would_remove_plugins = Vec::new();
    if kind == Some("claude") {
        for (plugin_name, companions) in CLAUDE_PLUGINS {
            if companions.contains(&skill)
                && !other_skill_on_root_needs_plugin(target_root, plugin_name, skill)
                && target_root.join(plugin_name).is_dir()
            {
                would_remove_plugins.push((*plugin_name).to_string());
            }
        }
    }

    let has_mcp_entry =
        integration::installed_mode(source_root, skill, &dest_dir).as_deref() == Some("mcp");

    let modified_files = digest::recorded_relative_paths(&dest_dir)
        .into_iter()
        .filter(|relative| {
            !digest::unmodified_since_install(&dest_dir, relative, &dest_dir.join(relative))
        })
        .collect();

    UninstallPreview {
        would_remove_shared_binaries,
        would_keep_shared_binaries,
        would_remove_plugins,
        has_mcp_entry,
        modified_files,
    }
}

/// Removes `skill` from `target_root`. `kind` is the agent kind this root
/// resolved to (`"claude"`/`"codex"`/`"opencode"`, or `None` for a custom/
/// unrecognized root) -- MCP deregistration needs it to know which CLI or
/// config file to touch; plugin GC only ever applies for `"claude"`, since
/// the other two have no per-root plugin copy.
pub fn uninstall_skill(
    source_root: &Path,
    skill: &str,
    target_root: &Path,
    home: &Path,
    kind: Option<&str>,
) -> io::Result<UninstallReport> {
    let dest_dir = target_root.join(skill);
    if !dest_dir.is_dir() {
        return Ok(UninstallReport {
            was_installed: false,
            removed_shared_binaries: Vec::new(),
            kept_shared_binaries: Vec::new(),
            removed_plugins: Vec::new(),
            mcp_entry_removed: false,
            modified_files: Vec::new(),
            permissions_removed: Vec::new(),
            opencode_tui_hint_plugin_removed: false,
        });
    }

    let needed = shared_binaries_needed_by(source_root, skill, &dest_dir);

    let modified_files: Vec<String> = digest::recorded_relative_paths(&dest_dir)
        .into_iter()
        .filter(|relative| {
            !digest::unmodified_since_install(&dest_dir, relative, &dest_dir.join(relative))
        })
        .collect();

    let mut mcp_entry_removed = false;
    if let Some(kind) = kind {
        if integration::installed_mode(source_root, skill, &dest_dir).as_deref() == Some("mcp") {
            let shared_dir = shared_bin::shared_bin_dir(home);
            mcp_entry_removed = mcp::unregister_for_kind(kind, skill, &shared_dir, home)?;
        }
    }

    fs::remove_dir_all(&dest_dir)?;

    let mut removed_shared_binaries = Vec::new();
    let mut kept_shared_binaries = Vec::new();
    for filename in &needed {
        if other_skill_needs_binary(source_root, home, filename, &dest_dir) {
            kept_shared_binaries.push(filename.clone());
            continue;
        }
        let path = shared_bin::shared_bin_dir(home).join(filename);
        if path.is_file() {
            fs::remove_file(&path)?;
        }
        removed_shared_binaries.push(filename.clone());
    }
    removed_shared_binaries.sort();
    kept_shared_binaries.sort();

    let mut removed_plugins = Vec::new();
    if kind == Some("claude") {
        for (plugin_name, companions) in CLAUDE_PLUGINS {
            if !companions.contains(&skill) {
                continue;
            }
            if other_skill_on_root_needs_plugin(target_root, plugin_name, skill) {
                continue;
            }
            let plugin_dir = target_root.join(plugin_name);
            if plugin_dir.is_dir() {
                fs::remove_dir_all(&plugin_dir)?;
                removed_plugins.push((*plugin_name).to_string());
            }
        }
    }

    let permissions_removed = match kind {
        Some(kind) => revoke_skill_permissions(skill, target_root, kind, home),
        None => Vec::new(),
    };

    let opencode_tui_hint_plugin_removed = kind == Some("opencode")
        && skill == "interactive-shell"
        && !other_root_has_interactive_shell(home, target_root)
        && crate::plugins::uninstall_tui_hint_plugin_opencode(home)?;

    Ok(UninstallReport {
        was_installed: true,
        removed_shared_binaries,
        kept_shared_binaries,
        removed_plugins,
        mcp_entry_removed,
        modified_files,
        permissions_removed,
        opencode_tui_hint_plugin_removed,
    })
}

/// Does some OTHER root this installer can discover under `home` still have
/// interactive-shell installed? Scoped the same way `other_skill_needs_binary`
/// already is (every known root, not just opencode ones) since a custom
/// location's agent kind is not recorded -- erring toward keeping the shared
/// plugin file when uncertain, never toward a false-positive deletion.
fn other_root_has_interactive_shell(home: &Path, exclude_root: &Path) -> bool {
    known_roots(home)
        .iter()
        .filter(|root| *root != exclude_root)
        .any(|root| root.join("interactive-shell").is_dir())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins;

    fn write(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    /// Seeds `source_root/<skill>` with a plain SKILL.md and a `bin/<host
    /// triple>/<binary>` entry, so `install::relative_paths_for` sees exactly
    /// one shared-binary row -- the same shape `plugins.rs`'s own tests seed.
    fn seed_skill_with_binary(source_root: &Path, skill: &str, binary: &str) {
        write(&source_root.join(skill).join("SKILL.md"), "content");
        let triple = installer_platform::current().unwrap();
        write(
            &source_root
                .join(skill)
                .join("bin")
                .join(triple.as_str())
                .join(binary),
            "#!/bin/sh\n",
        );
    }

    fn install_into(source_root: &Path, skill: &str, target_root: &Path, home: &Path) {
        install::install_skill(source_root, skill, target_root, home, None, true).unwrap();
    }

    #[test]
    fn a_uniquely_needed_shared_binary_is_removed() {
        let source_root = tempfile::tempdir().unwrap();
        seed_skill_with_binary(source_root.path(), "todo", "todo");
        let target_root = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        install_into(source_root.path(), "todo", target_root.path(), home.path());
        let shared_binary = shared_bin::shared_bin_dir(home.path()).join("todo");
        assert!(shared_binary.is_file());

        let report = uninstall_skill(
            source_root.path(),
            "todo",
            target_root.path(),
            home.path(),
            Some("claude"),
        )
        .unwrap();

        assert!(report.was_installed);
        assert_eq!(report.removed_shared_binaries, vec!["todo".to_string()]);
        assert!(report.kept_shared_binaries.is_empty());
        assert!(!target_root.path().join("todo").exists());
        assert!(!shared_binary.exists());
    }

    #[test]
    fn a_shared_binary_still_needed_by_a_sibling_root_is_kept() {
        let source_root = tempfile::tempdir().unwrap();
        // Two skills that happen to ship the same binary filename -- an
        // artificial setup (no two real skills collide today, T72's own plan
        // checked), but exactly what the reference count must key on: the
        // FILENAME, not the skill.
        seed_skill_with_binary(source_root.path(), "todo", "shared-tool");
        seed_skill_with_binary(source_root.path(), "bug-report", "shared-tool");
        let home = tempfile::tempdir().unwrap();
        let claude_root = home.path().join(".claude/skills");
        let codex_root = home.path().join(".codex/skills");
        install_into(source_root.path(), "todo", &claude_root, home.path());
        install_into(source_root.path(), "bug-report", &codex_root, home.path());
        let shared_binary = shared_bin::shared_bin_dir(home.path()).join("shared-tool");
        assert!(shared_binary.is_file());

        let report = uninstall_skill(
            source_root.path(),
            "todo",
            &claude_root,
            home.path(),
            Some("claude"),
        )
        .unwrap();

        assert!(report.removed_shared_binaries.is_empty());
        assert_eq!(report.kept_shared_binaries, vec!["shared-tool".to_string()]);
        assert!(
            shared_binary.is_file(),
            "bug-report on the codex root still needs it"
        );
    }

    #[test]
    fn a_plugin_still_needed_by_another_skill_on_the_same_root_is_kept() {
        let source_root = tempfile::tempdir().unwrap();
        write(&source_root.path().join("chat").join("SKILL.md"), "content");
        write(
            &source_root.path().join("ai-text-editor").join("SKILL.md"),
            "content",
        );
        let dir = source_root.path().join("agent-identity-plugin");
        write(&dir.join(".claude-plugin/plugin.json"), "{}");
        write(&dir.join("hooks/hooks.json"), "{}");
        write(&dir.join("hooks/lib.sh"), "#!/bin/sh\n");
        write(&dir.join("hooks/subagent-start.sh"), "#!/bin/sh\n");
        let target_root = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        install_into(source_root.path(), "chat", target_root.path(), home.path());
        install_into(
            source_root.path(),
            "ai-text-editor",
            target_root.path(),
            home.path(),
        );
        plugins::install_agent_identity_plugin_claude(source_root.path(), target_root.path())
            .unwrap();
        assert!(target_root.path().join("agent-identity-plugin").is_dir());

        let report = uninstall_skill(
            source_root.path(),
            "chat",
            target_root.path(),
            home.path(),
            Some("claude"),
        )
        .unwrap();

        assert!(report.removed_plugins.is_empty());
        assert!(
            target_root.path().join("agent-identity-plugin").is_dir(),
            "ai-text-editor on the same root still needs it"
        );

        let report = uninstall_skill(
            source_root.path(),
            "ai-text-editor",
            target_root.path(),
            home.path(),
            Some("claude"),
        )
        .unwrap();

        assert_eq!(
            report.removed_plugins,
            vec!["agent-identity-plugin".to_string()]
        );
        assert!(!target_root.path().join("agent-identity-plugin").exists());
    }

    #[test]
    fn an_absent_skill_reports_cleanly_rather_than_erroring() {
        let source_root = tempfile::tempdir().unwrap();
        let target_root = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();

        let report = uninstall_skill(
            source_root.path(),
            "todo",
            target_root.path(),
            home.path(),
            Some("claude"),
        )
        .unwrap();

        assert!(!report.was_installed);
        assert!(report.removed_shared_binaries.is_empty());
    }

    #[test]
    fn a_modified_file_is_reported_not_silently_discarded() {
        let source_root = tempfile::tempdir().unwrap();
        write(&source_root.path().join("todo").join("SKILL.md"), "content");
        let target_root = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        install_into(source_root.path(), "todo", target_root.path(), home.path());
        fs::write(
            target_root.path().join("todo").join("SKILL.md"),
            "user edited this",
        )
        .unwrap();

        let report = uninstall_skill(
            source_root.path(),
            "todo",
            target_root.path(),
            home.path(),
            Some("claude"),
        )
        .unwrap();

        assert_eq!(report.modified_files, vec!["SKILL.md".to_string()]);
    }

    #[test]
    fn uninstalling_interactive_shell_revokes_its_own_bin_execute_grant() {
        let source_root = tempfile::tempdir().unwrap();
        write(
            &source_root
                .path()
                .join("interactive-shell")
                .join("SKILL.md"),
            "content",
        );
        let home = tempfile::tempdir().unwrap();
        let claude_root = home.path().join(".claude/skills");
        install_into(
            source_root.path(),
            "interactive-shell",
            &claude_root,
            home.path(),
        );
        let settings = home.path().join(".claude").join("settings.json");
        fs::create_dir_all(settings.parent().unwrap()).unwrap();
        fs::write(&settings, "{}").unwrap();
        let bins = claude_root.join("interactive-shell").join("bin");
        crate::permissions::claude_interactive_shell_permissions(
            &bins.to_string_lossy(),
            home.path(),
        )
        .unwrap();

        let report = uninstall_skill(
            source_root.path(),
            "interactive-shell",
            &claude_root,
            home.path(),
            Some("claude"),
        )
        .unwrap();

        assert_eq!(report.permissions_removed.len(), 1);
        let settings = home.path().join(".claude").join("settings.json");
        let doc: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(settings).unwrap()).unwrap();
        assert!(doc["permissions"]["allow"].as_array().unwrap().is_empty());
    }

    #[test]
    fn uninstalling_a_skill_with_no_per_skill_grant_reports_no_permissions_removed() {
        let source_root = tempfile::tempdir().unwrap();
        write(&source_root.path().join("todo").join("SKILL.md"), "content");
        let target_root = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        install_into(source_root.path(), "todo", target_root.path(), home.path());

        let report = uninstall_skill(
            source_root.path(),
            "todo",
            target_root.path(),
            home.path(),
            Some("claude"),
        )
        .unwrap();

        assert!(report.permissions_removed.is_empty());
    }

    #[test]
    fn the_last_opencode_interactive_shell_removes_the_shared_tui_hint_plugin() {
        let source_root = tempfile::tempdir().unwrap();
        write(
            &source_root
                .path()
                .join("interactive-shell")
                .join("SKILL.md"),
            "content",
        );
        write(
            &source_root
                .path()
                .join("tui-hint-plugin/opencode/tui-hint-plugin.js"),
            "module.exports = {}\n",
        );
        let home = tempfile::tempdir().unwrap();
        let opencode_root = home.path().join(".config/opencode/skills");
        install_into(
            source_root.path(),
            "interactive-shell",
            &opencode_root,
            home.path(),
        );
        crate::plugins::install_tui_hint_plugin_opencode(source_root.path(), home.path()).unwrap();

        let report = uninstall_skill(
            source_root.path(),
            "interactive-shell",
            &opencode_root,
            home.path(),
            Some("opencode"),
        )
        .unwrap();

        assert!(report.opencode_tui_hint_plugin_removed);
        assert!(!crate::plugins::tui_hint_plugin_opencode_path(home.path()).is_file());
    }

    #[test]
    fn a_sibling_opencode_root_keeps_the_shared_tui_hint_plugin() {
        let source_root = tempfile::tempdir().unwrap();
        write(
            &source_root
                .path()
                .join("interactive-shell")
                .join("SKILL.md"),
            "content",
        );
        write(
            &source_root
                .path()
                .join("tui-hint-plugin/opencode/tui-hint-plugin.js"),
            "module.exports = {}\n",
        );
        let home = tempfile::tempdir().unwrap();
        let opencode_root = home.path().join(".config/opencode/skills");
        let custom_root = home.path().join("custom-opencode-root");
        install_into(
            source_root.path(),
            "interactive-shell",
            &opencode_root,
            home.path(),
        );
        install_into(
            source_root.path(),
            "interactive-shell",
            &custom_root,
            home.path(),
        );
        crate::custom_locations::save(home.path(), &custom_root).unwrap();
        crate::plugins::install_tui_hint_plugin_opencode(source_root.path(), home.path()).unwrap();

        let report = uninstall_skill(
            source_root.path(),
            "interactive-shell",
            &opencode_root,
            home.path(),
            Some("opencode"),
        )
        .unwrap();

        assert!(!report.opencode_tui_hint_plugin_removed);
        assert!(crate::plugins::tui_hint_plugin_opencode_path(home.path()).is_file());
    }
}
