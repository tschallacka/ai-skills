// MODE: DEV
// PACKAGE: PROD
//! Installs the two vendor-shipped plugins that ride along with a skill
//! rather than being selectable on their own -- ported from
//! installer/src/70-permissions.sh's `install_tui_hint_plugin_claude`/
//! `install_tui_hint_plugin_opencode`/`install_editor_gate_plugin`. Neither
//! is in manifest.rs's SKILLS list: tui-hint-plugin rides with
//! interactive-shell, editor-gate-plugin rides with ai-text-editor.
//!
//! Claude Code reads a plugin directory per root, so it is copied there
//! verbatim; opencode declares plugins globally in its own config's
//! `plugin` array (a local file path), so its copy lands once under this
//! installer's own XDG directory and is registered by path.

use crate::permissions;
use serde_json::Value;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

const TUI_HINT_PLUGIN_CLAUDE_FILES: &[&str] = &[
    ".claude-plugin/plugin.json",
    "hooks/hooks.json",
    "hooks/lib.sh",
    "hooks/pre-tool-use.sh",
];
const TUI_HINT_PLUGIN_CLAUDE_EXECUTABLES: &[&str] = &["hooks/lib.sh", "hooks/pre-tool-use.sh"];

const EDITOR_GATE_PLUGIN_FILES: &[&str] = &[
    ".claude-plugin/plugin.json",
    "hooks/hooks.json",
    "hooks/lib.sh",
    "hooks/editor-token",
    "hooks/pre-tool-use-bash.sh",
    "hooks/pre-tool-use-edit-write.sh",
];
const EDITOR_GATE_PLUGIN_EXECUTABLES: &[&str] = &[
    "hooks/editor-token",
    "hooks/lib.sh",
    "hooks/pre-tool-use-bash.sh",
    "hooks/pre-tool-use-edit-write.sh",
];

/// Copies `files` (relative to `source_root/plugin_name`) into
/// `target_root/plugin_name`, then makes `executables` (a subset of `files`)
/// executable on unix. A file the shipped tree does not have is silently
/// skipped, same as install.sh's `[ -f "$source" ] || continue`.
fn copy_plugin_files(
    source_root: &Path,
    plugin_name: &str,
    files: &[&str],
    executables: &[&str],
    target_root: &Path,
) -> io::Result<PathBuf> {
    let source_plugin_dir = source_root.join(plugin_name);
    let destination = target_root.join(plugin_name);
    for relative in files {
        let source = source_plugin_dir.join(relative);
        if !source.is_file() {
            continue;
        }
        let dest_file = destination.join(relative);
        if let Some(parent) = dest_file.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(&source, &dest_file)?;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for relative in executables {
            let dest_file = destination.join(relative);
            if !dest_file.is_file() {
                continue;
            }
            let mut perms = fs::metadata(&dest_file)?.permissions();
            perms.set_mode(perms.mode() | 0o111);
            fs::set_permissions(&dest_file, perms)?;
        }
    }
    Ok(destination)
}

pub fn install_tui_hint_plugin_claude(
    source_root: &Path,
    target_root: &Path,
) -> io::Result<PathBuf> {
    copy_plugin_files(
        source_root,
        "tui-hint-plugin",
        TUI_HINT_PLUGIN_CLAUDE_FILES,
        TUI_HINT_PLUGIN_CLAUDE_EXECUTABLES,
        target_root,
    )
}

pub fn install_editor_gate_plugin(source_root: &Path, target_root: &Path) -> io::Result<PathBuf> {
    copy_plugin_files(
        source_root,
        "editor-gate-plugin",
        EDITOR_GATE_PLUGIN_FILES,
        EDITOR_GATE_PLUGIN_EXECUTABLES,
        target_root,
    )
}

fn xdg_config_home(home: &Path) -> PathBuf {
    std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"))
}

pub enum OpencodePluginOutcome {
    /// This run's checkout does not ship the opencode variant of the plugin.
    NotShipped,
    NotStrictJson,
    AlreadyRegistered,
    Registered,
}

/// Ensures `entry` is present in the config's `plugin` array (a bare local
/// file path, opencode's own shape for one), never duplicating it.
fn register_plugin_entry(cfg: &Path, entry: &str) -> io::Result<bool> {
    let raw = fs::read_to_string(cfg)?;
    let mut doc = permissions::as_object(serde_json::from_str(&raw).ok());
    let existing = match doc.get("plugin") {
        Some(Value::Array(items)) => items.clone(),
        _ => Vec::new(),
    };
    if existing.iter().any(|v| v.as_str() == Some(entry)) {
        return Ok(false);
    }
    let mut updated = existing;
    updated.push(Value::String(entry.to_string()));
    doc.insert("plugin".to_string(), Value::Array(updated));
    permissions::write_preserving_mode(cfg, &serde_json::to_string_pretty(&Value::Object(doc))?)?;
    Ok(true)
}

pub fn install_tui_hint_plugin_opencode(
    source_root: &Path,
    home: &Path,
) -> io::Result<OpencodePluginOutcome> {
    let source = source_root
        .join("tui-hint-plugin")
        .join("opencode")
        .join("tui-hint-plugin.js");
    if !source.is_file() {
        return Ok(OpencodePluginOutcome::NotShipped);
    }
    let destination = xdg_config_home(home)
        .join("tsch-ai-skills")
        .join("tui-hint-plugin")
        .join("tui-hint-plugin.js");
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(&source, &destination)?;

    let cfg = permissions::opencode_configfile(home);
    if permissions::opencode_prepare_config(&cfg)?.is_none() {
        return Ok(OpencodePluginOutcome::NotStrictJson);
    }
    let added = register_plugin_entry(&cfg, &destination.to_string_lossy())?;
    Ok(if added {
        OpencodePluginOutcome::Registered
    } else {
        OpencodePluginOutcome::AlreadyRegistered
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    fn seed_tui_hint_plugin_claude(source_root: &Path) {
        let dir = source_root.join("tui-hint-plugin");
        write(&dir.join(".claude-plugin/plugin.json"), "{}");
        write(&dir.join("hooks/hooks.json"), "{}");
        write(&dir.join("hooks/lib.sh"), "#!/bin/sh\n");
        write(&dir.join("hooks/pre-tool-use.sh"), "#!/bin/sh\n");
        write(&dir.join("README.md"), "not shipped");
    }

    #[test]
    fn tui_hint_plugin_claude_copies_only_the_needed_files_and_marks_hooks_executable() {
        let source_root = tempfile::tempdir().unwrap();
        seed_tui_hint_plugin_claude(source_root.path());
        let target_root = tempfile::tempdir().unwrap();

        let destination =
            install_tui_hint_plugin_claude(source_root.path(), target_root.path()).unwrap();

        assert!(destination.join(".claude-plugin/plugin.json").is_file());
        assert!(destination.join("hooks/pre-tool-use.sh").is_file());
        assert!(!destination.join("README.md").exists());

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(destination.join("hooks/lib.sh"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o111, 0o111);
        }
    }

    #[test]
    fn a_missing_source_file_is_skipped_not_refused() {
        let source_root = tempfile::tempdir().unwrap();
        // Ship only plugin.json; the rest is absent.
        write(
            &source_root
                .path()
                .join("tui-hint-plugin/.claude-plugin/plugin.json"),
            "{}",
        );
        let target_root = tempfile::tempdir().unwrap();

        let destination =
            install_tui_hint_plugin_claude(source_root.path(), target_root.path()).unwrap();
        assert!(destination.join(".claude-plugin/plugin.json").is_file());
        assert!(!destination.join("hooks/lib.sh").exists());
    }

    #[test]
    fn editor_gate_plugin_copies_its_own_file_set() {
        let source_root = tempfile::tempdir().unwrap();
        let dir = source_root.path().join("editor-gate-plugin");
        write(&dir.join(".claude-plugin/plugin.json"), "{}");
        write(&dir.join("hooks/hooks.json"), "{}");
        write(&dir.join("hooks/lib.sh"), "#!/bin/sh\n");
        write(&dir.join("hooks/editor-token"), "#!/bin/sh\n");
        write(&dir.join("hooks/pre-tool-use-bash.sh"), "#!/bin/sh\n");
        write(&dir.join("hooks/pre-tool-use-edit-write.sh"), "#!/bin/sh\n");
        let target_root = tempfile::tempdir().unwrap();

        let destination =
            install_editor_gate_plugin(source_root.path(), target_root.path()).unwrap();
        assert!(destination.join("hooks/editor-token").is_file());

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = fs::metadata(destination.join("hooks/editor-token"))
                .unwrap()
                .permissions()
                .mode();
            assert_eq!(mode & 0o111, 0o111);
        }
    }

    #[test]
    fn opencode_variant_not_shipped_is_reported_without_writing_anything() {
        let source_root = tempfile::tempdir().unwrap();
        let home = tempfile::tempdir().unwrap();
        let outcome = install_tui_hint_plugin_opencode(source_root.path(), home.path()).unwrap();
        assert!(matches!(outcome, OpencodePluginOutcome::NotShipped));
    }

    #[test]
    fn opencode_variant_registers_the_plugin_path_once() {
        let source_root = tempfile::tempdir().unwrap();
        write(
            &source_root
                .path()
                .join("tui-hint-plugin/opencode/tui-hint-plugin.js"),
            "module.exports = {}\n",
        );
        let home = tempfile::tempdir().unwrap();

        let first = install_tui_hint_plugin_opencode(source_root.path(), home.path()).unwrap();
        assert!(matches!(first, OpencodePluginOutcome::Registered));

        let second = install_tui_hint_plugin_opencode(source_root.path(), home.path()).unwrap();
        assert!(matches!(second, OpencodePluginOutcome::AlreadyRegistered));

        let cfg = permissions::opencode_configfile(home.path());
        let doc: Value = serde_json::from_str(&fs::read_to_string(cfg).unwrap()).unwrap();
        assert_eq!(doc["plugin"].as_array().unwrap().len(), 1);
    }
}
