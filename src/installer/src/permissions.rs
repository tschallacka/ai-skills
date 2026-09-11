// MODE: DEV
// PACKAGE: PROD
//! Grants Claude Code permission to touch the planning skill's own scripts,
//! plan root, and tmp directory without a per-call prompt -- ported from
//! installer/src/70-permissions.sh's `claude_permissions`/`claude_merge_allow`.
//!
//! install.sh shells out to `rjq` to edit `~/.claude/settings.json`; this
//! installer is Rust already, so it edits the JSON directly with serde_json
//! instead of spawning a JSON tool. The merge semantics are kept identical:
//! non-object JSON (or an unparsable file) reads as `{}`, existing
//! `permissions.allow` entries are kept in place and never duplicated, and
//! new ones are appended in a fixed order. Only agents other than Claude Code
//! (opencode, codex) and the other permission grants (worktrees, planning
//! interactive-shell, tui-hint-plugin, editor-steering/-gate) remain unported.

use crate::backup;
use serde_json::{Map, Value};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

pub enum PermissionOutcome {
    /// No settings.json exists at the resolved path; nothing was touched.
    NoConfigFile,
    /// Every entry this grant needs was already present.
    AlreadyPresent,
    /// These entries were appended to `permissions.allow`.
    Added(Vec<String>),
}

fn strip_trailing_slashes(value: &str) -> &str {
    value.trim_end_matches('/')
}

fn claude_settings_path(home: &Path) -> PathBuf {
    home.join(".claude").join("settings.json")
}

/// The eight entries install.sh's `claude_permissions` grants for the
/// planning skill: read/write on the plan root and the tmp directory, plus
/// read and execute (both a direct and a `bash `-prefixed form) on the
/// installed planning scripts directory.
fn planning_entries(scripts: &str, plans: &str, tmp: &str) -> Vec<String> {
    vec![
        format!("Read({plans}/**)"),
        format!("Edit({plans}/**)"),
        format!("Bash({scripts}/**:*)"),
        format!("Read({scripts}/**)"),
        format!("Bash(bash {scripts}/**:*)"),
        format!("Read({tmp}/**)"),
        format!("Edit({tmp}/**)"),
        format!("Bash({tmp}/**:*)"),
    ]
}

pub fn claude_planning_permissions(
    scripts: &str,
    plans: &str,
    tmp: &str,
    home: &Path,
) -> io::Result<PermissionOutcome> {
    let cfg = claude_settings_path(home);
    if !cfg.is_file() {
        return Ok(PermissionOutcome::NoConfigFile);
    }
    let entries = planning_entries(
        strip_trailing_slashes(scripts),
        strip_trailing_slashes(plans),
        strip_trailing_slashes(tmp),
    );
    merge_allow_entries(&cfg, &entries)
}

/// A JSON document read as an object, same as install.sh's `objectify`:
/// anything that isn't already an object (a scalar, an array, or a file that
/// failed to parse at all) reads as `{}` rather than refusing.
fn as_object(value: Option<Value>) -> Map<String, Value> {
    match value {
        Some(Value::Object(map)) => map,
        _ => Map::new(),
    }
}

fn allow_array(permissions: &Map<String, Value>) -> Vec<Value> {
    match permissions.get("allow") {
        Some(Value::Array(items)) => items.clone(),
        _ => Vec::new(),
    }
}

fn merge_allow_entries(cfg: &Path, entries: &[String]) -> io::Result<PermissionOutcome> {
    backup::backup_file(cfg)?;

    let raw = fs::read_to_string(cfg)?;
    let mut doc = as_object(serde_json::from_str(&raw).ok());
    let mut permissions = as_object(doc.get("permissions").cloned());
    let allow = allow_array(&permissions);

    let already_present = |value: &Value| allow.contains(value);
    let added: Vec<String> = entries
        .iter()
        .filter(|e| !already_present(&Value::String((*e).clone())))
        .cloned()
        .collect();
    if added.is_empty() {
        return Ok(PermissionOutcome::AlreadyPresent);
    }

    let mut new_allow = allow;
    new_allow.extend(added.iter().cloned().map(Value::String));
    permissions.insert("allow".to_string(), Value::Array(new_allow));
    doc.insert("permissions".to_string(), Value::Object(permissions));

    write_preserving_mode(cfg, &serde_json::to_string_pretty(&Value::Object(doc))?)?;
    Ok(PermissionOutcome::Added(added))
}

/// `cp -p`'s effect, in the atomic-write shape the rest of this installer
/// uses: the replacement file keeps the original's permission bits rather
/// than whatever `fs::write` on a new file would default to.
fn write_preserving_mode(dest: &Path, content: &str) -> io::Result<()> {
    let file_name = dest.file_name().unwrap_or_default().to_string_lossy();
    let temp = dest.with_file_name(format!(".{file_name}.installer-tmp.{}", std::process::id()));
    fs::write(&temp, content)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = fs::metadata(dest)?.permissions().mode();
        let mut perms = fs::metadata(&temp)?.permissions();
        perms.set_mode(mode);
        fs::set_permissions(&temp, perms)?;
    }
    fs::rename(&temp, dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings_at(home: &Path, content: &str) -> PathBuf {
        let dir = home.join(".claude");
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("settings.json");
        fs::write(&path, content).unwrap();
        path
    }

    #[test]
    fn no_settings_file_is_reported_and_nothing_is_written() {
        let home = tempfile::tempdir().unwrap();
        let outcome = claude_planning_permissions("s", "p", "t", home.path()).unwrap();
        assert!(matches!(outcome, PermissionOutcome::NoConfigFile));
    }

    #[test]
    fn a_fresh_settings_file_gains_all_eight_entries() {
        let home = tempfile::tempdir().unwrap();
        let cfg = settings_at(home.path(), "{}");

        let outcome =
            claude_planning_permissions("/scripts", "/plans", "/tmp", home.path()).unwrap();
        let added = match outcome {
            PermissionOutcome::Added(entries) => entries,
            _ => panic!("expected Added"),
        };
        assert_eq!(added.len(), 8);
        assert!(added.contains(&"Read(/plans/**)".to_string()));
        assert!(added.contains(&"Bash(bash /scripts/**:*)".to_string()));

        let doc: Value = serde_json::from_str(&fs::read_to_string(&cfg).unwrap()).unwrap();
        let allow = doc["permissions"]["allow"].as_array().unwrap();
        assert_eq!(allow.len(), 8);
    }

    #[test]
    fn a_second_grant_adds_nothing_and_reports_already_present() {
        let home = tempfile::tempdir().unwrap();
        settings_at(home.path(), "{}");
        claude_planning_permissions("/scripts", "/plans", "/tmp", home.path()).unwrap();

        let outcome =
            claude_planning_permissions("/scripts", "/plans", "/tmp", home.path()).unwrap();
        assert!(matches!(outcome, PermissionOutcome::AlreadyPresent));
    }

    #[test]
    fn existing_unrelated_allow_entries_and_settings_survive_the_merge() {
        let home = tempfile::tempdir().unwrap();
        let cfg = settings_at(
            home.path(),
            r#"{"model":"opus","permissions":{"allow":["Bash(ls:*)"]}}"#,
        );

        claude_planning_permissions("/scripts", "/plans", "/tmp", home.path()).unwrap();

        let doc: Value = serde_json::from_str(&fs::read_to_string(&cfg).unwrap()).unwrap();
        assert_eq!(doc["model"], "opus");
        let allow = doc["permissions"]["allow"].as_array().unwrap();
        assert_eq!(allow[0], "Bash(ls:*)");
        assert_eq!(allow.len(), 9);
    }

    #[test]
    fn trailing_slashes_on_the_input_paths_do_not_duplicate_entries() {
        let home = tempfile::tempdir().unwrap();
        settings_at(home.path(), "{}");
        claude_planning_permissions("/scripts/", "/plans/", "/tmp/", home.path()).unwrap();

        let outcome =
            claude_planning_permissions("/scripts", "/plans", "/tmp", home.path()).unwrap();
        assert!(matches!(outcome, PermissionOutcome::AlreadyPresent));
    }

    #[test]
    fn unparsable_json_is_treated_as_an_empty_document_rather_than_refused() {
        let home = tempfile::tempdir().unwrap();
        settings_at(home.path(), "not json at all");

        let outcome =
            claude_planning_permissions("/scripts", "/plans", "/tmp", home.path()).unwrap();
        assert!(matches!(outcome, PermissionOutcome::Added(_)));
    }

    #[cfg(unix)]
    #[test]
    fn the_settings_files_permission_bits_survive_the_rewrite() {
        use std::os::unix::fs::PermissionsExt;
        let home = tempfile::tempdir().unwrap();
        let cfg = settings_at(home.path(), "{}");
        fs::set_permissions(&cfg, fs::Permissions::from_mode(0o600)).unwrap();

        claude_planning_permissions("/scripts", "/plans", "/tmp", home.path()).unwrap();

        let mode = fs::metadata(&cfg).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }
}
