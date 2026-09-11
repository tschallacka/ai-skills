// MODE: DEV
// PACKAGE: PROD
//! Grants Claude Code and opencode permission to touch the planning skill's
//! own scripts, plan root, and tmp directory without a per-call prompt --
//! ported from installer/src/70-permissions.sh's `claude_permissions`/
//! `claude_merge_allow` and `opencode_permissions`/`opencode_merge_permission`.
//!
//! install.sh shells out to `rjq` to edit each agent's JSON config; this
//! installer is Rust already, so it edits the JSON directly with serde_json
//! instead of spawning a JSON tool. The merge semantics are kept identical
//! for each agent's own config shape. Still unported: codex (config.toml,
//! not JSON), the worktrees/interactive-shell/tui-hint-plugin/editor-
//! steering grants, and opencode's own worktree variant.

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

/// No separate `Write(...)` entry: Claude Code's permission engine has no
/// rule keyed on the Write tool and does not fall back to a matching
/// `Edit(...)` rule either -- `Edit(path)` is the umbrella that already
/// covers every file-editing tool, Write included.
pub fn claude_worktrees_permissions(worktrees: &str, home: &Path) -> io::Result<PermissionOutcome> {
    let cfg = claude_settings_path(home);
    if !cfg.is_file() {
        return Ok(PermissionOutcome::NoConfigFile);
    }
    let worktrees = strip_trailing_slashes(worktrees);
    let entries = vec![
        format!("Read({worktrees}/**)"),
        format!("Edit({worktrees}/**)"),
        format!("Bash({worktrees}/**:*)"),
    ];
    merge_allow_entries(&cfg, &entries)
}

/// A JSON document read as an object, same as install.sh's `objectify`:
/// anything that isn't already an object (a scalar, an array, or a file that
/// failed to parse at all) reads as `{}` rather than refusing.
pub(crate) fn as_object(value: Option<Value>) -> Map<String, Value> {
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
pub(crate) fn write_preserving_mode(dest: &Path, content: &str) -> io::Result<()> {
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

// ---------------------------------------------------------------
// opencode
// ---------------------------------------------------------------

pub enum OpencodePermissionOutcome {
    /// The config exists, is non-empty, and does not parse as JSON (JSON-C
    /// comments or a trailing comma) -- rewriting it would strip content the
    /// user wrote, so nothing was touched.
    NotStrictJson,
    /// `legacy_removed` and `added` are reported independently, same as
    /// install.sh's two separate print statements: a stray Claude-style
    /// `permission.allow` array (not a valid opencode shape) can be dropped
    /// on the very same run that also adds fresh rules, or on a run that
    /// adds nothing at all.
    Merged {
        legacy_removed: bool,
        added: Vec<String>,
    },
}

pub(crate) fn opencode_configfile(home: &Path) -> PathBuf {
    if let Ok(explicit) = std::env::var("OPENCODE_CONFIGFILE") {
        return PathBuf::from(explicit);
    }
    let dir = home.join(".config").join("opencode");
    let json = dir.join("opencode.json");
    let jsonc = dir.join("opencode.jsonc");
    if json.is_file() || !jsonc.is_file() {
        json
    } else {
        jsonc
    }
}

/// Resolves the config, creating a minimal one if missing, and backs it up
/// otherwise -- mirrors install.sh's `opencode_prepare_config`. Returns
/// `None` (having touched nothing) when an existing, non-empty file is not
/// strict JSON.
fn opencode_prepare_config(cfg: &Path) -> io::Result<Option<()>> {
    if !cfg.is_file() {
        if let Some(parent) = cfg.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(
            cfg,
            "{\n  \"$schema\": \"https://opencode.ai/config.json\"\n}\n",
        )?;
        return Ok(Some(()));
    }
    let raw = fs::read_to_string(cfg)?;
    if !raw.trim().is_empty() && serde_json::from_str::<Value>(&raw).is_err() {
        return Ok(None);
    }
    backup::backup_file(cfg)?;
    Ok(Some(()))
}

/// `rules` in 70-permissions.sh: a tool's own permission entry, normalized
/// to a pattern->decision map. A bare string (opencode's shorthand for "this
/// decision for every pattern") becomes a single `"*"` entry.
fn rules_of(value: Option<&Value>) -> Map<String, Value> {
    match value {
        Some(Value::Object(map)) => map.clone(),
        Some(Value::String(s)) => {
            let mut m = Map::new();
            m.insert("*".to_string(), Value::String(s.clone()));
            m
        }
        _ => Map::new(),
    }
}

/// `base` in 70-permissions.sh: the existing `.permission` block, with any
/// top-level Claude-style `allow`/`deny`/`ask` keys dropped, and a bare
/// string shorthand spread across only the tools this grant cares about.
fn base_permission(permission: Option<&Value>, wanted_tools: &[&str]) -> Map<String, Value> {
    match permission {
        Some(Value::String(s)) => wanted_tools
            .iter()
            .map(|&tool| {
                let mut rule = Map::new();
                rule.insert("*".to_string(), Value::String(s.clone()));
                (tool.to_string(), Value::Object(rule))
            })
            .collect(),
        Some(Value::Object(map)) => {
            let mut m = map.clone();
            m.remove("allow");
            m.remove("deny");
            m.remove("ask");
            m
        }
        _ => Map::new(),
    }
}

fn has_legacy_allow_list(permission: Option<&Value>) -> bool {
    matches!(
        permission.and_then(|p| p.as_object()).and_then(|p| p.get("allow")),
        Some(Value::Array(items)) if !items.is_empty()
    )
}

/// One (tool, patterns-to-allow) pair, e.g. `("read", &["/plans/**"])`.
type WantedRule<'a> = (&'a str, &'a [String]);

fn opencode_merge_permission(
    cfg: &Path,
    wanted: &[WantedRule],
) -> io::Result<OpencodePermissionOutcome> {
    let raw = fs::read_to_string(cfg)?;
    let doc = as_object(serde_json::from_str(&raw).ok());
    let tools: Vec<&str> = wanted.iter().map(|(tool, _)| *tool).collect();
    let legacy = has_legacy_allow_list(doc.get("permission"));
    let base = base_permission(doc.get("permission"), &tools);

    let mut added = Vec::new();
    let mut perm = base.clone();
    for (tool, patterns) in wanted {
        let mut rule = rules_of(base.get(*tool));
        for pattern in *patterns {
            if rule.get(pattern).and_then(Value::as_str) != Some("allow") {
                added.push(format!("{tool}: {pattern}"));
            }
            rule.insert(pattern.clone(), Value::String("allow".to_string()));
        }
        perm.insert(tool.to_string(), Value::Object(rule));
    }

    let mut doc = doc;
    doc.insert("permission".to_string(), Value::Object(perm));
    write_preserving_mode(cfg, &serde_json::to_string_pretty(&Value::Object(doc))?)?;

    Ok(OpencodePermissionOutcome::Merged {
        legacy_removed: legacy,
        added,
    })
}

pub fn opencode_planning_permissions(
    scripts: &str,
    plans: &str,
    tmp: &str,
    home: &Path,
) -> io::Result<OpencodePermissionOutcome> {
    let cfg = opencode_configfile(home);
    if opencode_prepare_config(&cfg)?.is_none() {
        return Ok(OpencodePermissionOutcome::NotStrictJson);
    }
    let plans = strip_trailing_slashes(plans).to_string();
    let scripts = strip_trailing_slashes(scripts).to_string();
    let tmp = strip_trailing_slashes(tmp).to_string();

    let read = vec![
        format!("{plans}/**"),
        format!("{scripts}/**"),
        format!("{tmp}/**"),
    ];
    let edit = vec![format!("{plans}/**"), format!("{tmp}/**")];
    let bash = vec![
        format!("{scripts}/**"),
        format!("bash {scripts}/**"),
        format!("{tmp}/**"),
    ];
    let external_directory = vec![
        format!("{plans}/**"),
        format!("{scripts}/**"),
        format!("{tmp}/**"),
    ];
    opencode_merge_permission(
        &cfg,
        &[
            ("read", &read),
            ("edit", &edit),
            ("bash", &bash),
            ("external_directory", &external_directory),
        ],
    )
}

pub fn opencode_worktrees_permissions(
    worktrees: &str,
    home: &Path,
) -> io::Result<OpencodePermissionOutcome> {
    let cfg = opencode_configfile(home);
    if opencode_prepare_config(&cfg)?.is_none() {
        return Ok(OpencodePermissionOutcome::NotStrictJson);
    }
    let pattern = vec![format!("{}/**", strip_trailing_slashes(worktrees))];
    opencode_merge_permission(
        &cfg,
        &[
            ("read", &pattern),
            ("edit", &pattern),
            ("write", &pattern),
            ("bash", &pattern),
            ("external_directory", &pattern),
        ],
    )
}

// ---------------------------------------------------------------
// codex
// ---------------------------------------------------------------
//
// codex reads ~/.codex/config.toml, which is TOML, not JSON: this handles
// exactly one well-defined shape, a single-line `writable_roots = [...]`
// array wherever it appears (a root-level dotted key or inside a
// `[sandbox_workspace_write]` table look identical on the matching line, so
// one search covers both). A multi-line array falls back to
// `NotSingleLineArray`, same as install.sh's manual-instructions fallback.

pub enum CodexOutcome {
    /// No config file existed; one was written with just this array.
    Created,
    /// The config existed with no `writable_roots` line; one was prepended
    /// (TOML's dotted-key syntax only reliably names a root-level key while
    /// no `[table]` header has been opened yet, so prepending -- never
    /// appending after whatever section happens to be last -- is the only
    /// placement guaranteed to land at the root).
    Prepended,
    AlreadyPresent,
    Appended(Vec<String>),
    /// The line exists but is not a single `[...]` array on one line; this
    /// installer refuses to guess how to extend it.
    NotSingleLineArray,
}

pub(crate) fn codex_configfile(home: &Path) -> PathBuf {
    std::env::var("CODEX_CONFIGFILE")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home.join(".codex").join("config.toml"))
}

fn quoted_csv(paths: &[String]) -> String {
    paths
        .iter()
        .map(|p| format!("\"{p}\""))
        .collect::<Vec<_>>()
        .join(", ")
}

fn writable_roots_line(content: &str) -> Option<usize> {
    let re = regex::Regex::new(r"writable_roots\s*=").expect("static regex");
    content.lines().position(|line| re.is_match(line))
}

fn rejoin_preserving_trailing_newline(original: &str, lines: Vec<String>) -> String {
    let mut out = lines.join("\n");
    if original.ends_with('\n') {
        out.push('\n');
    }
    out
}

fn write_fresh_roots(cfg: &Path, wanted: &[String]) -> io::Result<CodexOutcome> {
    let line = format!(
        "sandbox_workspace_write.writable_roots = [{}]\n",
        quoted_csv(wanted)
    );
    if !cfg.is_file() {
        if let Some(parent) = cfg.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(cfg, line)?;
        return Ok(CodexOutcome::Created);
    }
    backup::backup_file(cfg)?;
    let existing = fs::read_to_string(cfg)?;
    write_preserving_mode(cfg, &format!("{line}{existing}"))?;
    Ok(CodexOutcome::Prepended)
}

fn codex_merge_writable_roots(cfg: &Path, wanted: &[String]) -> io::Result<CodexOutcome> {
    if !cfg.is_file() {
        return write_fresh_roots(cfg, wanted);
    }
    let content = fs::read_to_string(cfg)?;
    let Some(line_index) = writable_roots_line(&content) else {
        return write_fresh_roots(cfg, wanted);
    };
    let mut lines: Vec<String> = content.lines().map(str::to_string).collect();
    let line = lines[line_index].clone();
    if !(line.contains('[') && line.contains(']')) {
        return Ok(CodexOutcome::NotSingleLineArray);
    }
    let to_add: Vec<String> = wanted
        .iter()
        .filter(|p| !line.contains(&format!("\"{p}\"")))
        .cloned()
        .collect();
    if to_add.is_empty() {
        return Ok(CodexOutcome::AlreadyPresent);
    }
    backup::backup_file(cfg)?;
    let (before, after) = line.split_once(']').unwrap_or((line.as_str(), ""));
    lines[line_index] = format!("{before}, {}]{after}", quoted_csv(&to_add));
    write_preserving_mode(cfg, &rejoin_preserving_trailing_newline(&content, lines))?;
    Ok(CodexOutcome::Appended(to_add))
}

pub fn codex_planning_permissions(
    scripts: &str,
    plans: &str,
    tmp: &str,
    home: &Path,
) -> io::Result<CodexOutcome> {
    let cfg = codex_configfile(home);
    let wanted = vec![
        strip_trailing_slashes(plans).to_string(),
        strip_trailing_slashes(scripts).to_string(),
        strip_trailing_slashes(tmp).to_string(),
    ];
    codex_merge_writable_roots(&cfg, &wanted)
}

pub fn codex_worktrees_permissions(worktrees: &str, home: &Path) -> io::Result<CodexOutcome> {
    let cfg = codex_configfile(home);
    let wanted = vec![strip_trailing_slashes(worktrees).to_string()];
    codex_merge_writable_roots(&cfg, &wanted)
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

    fn opencode_cfg(home: &Path) -> PathBuf {
        home.join(".config").join("opencode").join("opencode.json")
    }

    #[test]
    fn a_missing_opencode_config_is_created_and_gains_every_rule() {
        let home = tempfile::tempdir().unwrap();

        let outcome =
            opencode_planning_permissions("/scripts", "/plans", "/tmp", home.path()).unwrap();

        let (legacy_removed, added) = match outcome {
            OpencodePermissionOutcome::Merged {
                legacy_removed,
                added,
            } => (legacy_removed, added),
            _ => panic!("expected Merged"),
        };
        assert!(!legacy_removed);
        // read(3) + edit(2) + bash(3) + external_directory(3)
        assert_eq!(added.len(), 11);
        assert!(added.contains(&"read: /plans/**".to_string()));
        assert!(added.contains(&"bash: bash /scripts/**".to_string()));

        let cfg = opencode_cfg(home.path());
        let doc: Value = serde_json::from_str(&fs::read_to_string(&cfg).unwrap()).unwrap();
        assert_eq!(doc["permission"]["read"]["/plans/**"], "allow");
    }

    #[test]
    fn a_second_opencode_grant_reports_already_present() {
        let home = tempfile::tempdir().unwrap();
        opencode_planning_permissions("/scripts", "/plans", "/tmp", home.path()).unwrap();

        let outcome =
            opencode_planning_permissions("/scripts", "/plans", "/tmp", home.path()).unwrap();
        match outcome {
            OpencodePermissionOutcome::Merged {
                legacy_removed,
                added,
            } => {
                assert!(!legacy_removed);
                assert!(added.is_empty());
            }
            _ => panic!("expected Merged"),
        }
    }

    #[test]
    fn not_strict_json_is_left_untouched() {
        let home = tempfile::tempdir().unwrap();
        let cfg = opencode_cfg(home.path());
        fs::create_dir_all(cfg.parent().unwrap()).unwrap();
        fs::write(&cfg, "{ // a comment\n}\n").unwrap();
        let before = fs::read_to_string(&cfg).unwrap();

        let outcome =
            opencode_planning_permissions("/scripts", "/plans", "/tmp", home.path()).unwrap();

        assert!(matches!(outcome, OpencodePermissionOutcome::NotStrictJson));
        assert_eq!(fs::read_to_string(&cfg).unwrap(), before);
    }

    #[test]
    fn a_legacy_claude_style_allow_list_is_dropped_and_reported() {
        let home = tempfile::tempdir().unwrap();
        let cfg = opencode_cfg(home.path());
        fs::create_dir_all(cfg.parent().unwrap()).unwrap();
        fs::write(&cfg, r#"{"permission":{"allow":["Bash(ls:*)"]}}"#).unwrap();

        let outcome =
            opencode_planning_permissions("/scripts", "/plans", "/tmp", home.path()).unwrap();
        match outcome {
            OpencodePermissionOutcome::Merged {
                legacy_removed,
                added,
            } => {
                assert!(legacy_removed);
                assert_eq!(added.len(), 11);
            }
            _ => panic!("expected Merged"),
        }
        let doc: Value = serde_json::from_str(&fs::read_to_string(&cfg).unwrap()).unwrap();
        assert!(doc["permission"].get("allow").is_none());

        // The stray key is gone now, so a rerun reports no further removal.
        let outcome =
            opencode_planning_permissions("/scripts", "/plans", "/tmp", home.path()).unwrap();
        match outcome {
            OpencodePermissionOutcome::Merged { legacy_removed, .. } => assert!(!legacy_removed),
            _ => panic!("expected Merged"),
        }
    }

    #[test]
    fn an_existing_string_shorthand_permission_is_preserved_for_other_tools() {
        let home = tempfile::tempdir().unwrap();
        let cfg = opencode_cfg(home.path());
        fs::create_dir_all(cfg.parent().unwrap()).unwrap();
        fs::write(&cfg, r#"{"permission":"ask"}"#).unwrap();

        opencode_planning_permissions("/scripts", "/plans", "/tmp", home.path()).unwrap();

        let doc: Value = serde_json::from_str(&fs::read_to_string(&cfg).unwrap()).unwrap();
        // The shorthand seeded every wanted tool with {"*": "ask"}, and the
        // merge then adds "allow" only for the specific patterns requested.
        assert_eq!(doc["permission"]["read"]["*"], "ask");
        assert_eq!(doc["permission"]["read"]["/plans/**"], "allow");
    }

    #[test]
    fn claude_worktrees_permissions_grants_three_entries() {
        let home = tempfile::tempdir().unwrap();
        let cfg = settings_at(home.path(), "{}");

        let outcome = claude_worktrees_permissions("/wt", home.path()).unwrap();
        let added = match outcome {
            PermissionOutcome::Added(entries) => entries,
            _ => panic!("expected Added"),
        };
        assert_eq!(added.len(), 3);
        let doc: Value = serde_json::from_str(&fs::read_to_string(&cfg).unwrap()).unwrap();
        let allow = doc["permissions"]["allow"].as_array().unwrap();
        assert!(allow.iter().any(|v| v == "Bash(/wt/**:*)"));
        assert!(!allow
            .iter()
            .any(|v| v.as_str().unwrap().starts_with("Write(")));
    }

    #[test]
    fn opencode_worktrees_permissions_grants_five_tools() {
        let home = tempfile::tempdir().unwrap();
        let outcome = opencode_worktrees_permissions("/wt", home.path()).unwrap();
        let added = match outcome {
            OpencodePermissionOutcome::Merged { added, .. } => added,
            _ => panic!("expected Merged"),
        };
        assert_eq!(added.len(), 5);
        assert!(added.contains(&"write: /wt/**".to_string()));
    }

    fn codex_cfg(home: &Path) -> PathBuf {
        home.join(".codex").join("config.toml")
    }

    #[test]
    fn a_missing_codex_config_is_created_with_the_roots_array() {
        let home = tempfile::tempdir().unwrap();
        let outcome =
            codex_planning_permissions("/scripts", "/plans", "/tmp", home.path()).unwrap();
        assert!(matches!(outcome, CodexOutcome::Created));

        let content = fs::read_to_string(codex_cfg(home.path())).unwrap();
        assert_eq!(
            content,
            "sandbox_workspace_write.writable_roots = [\"/plans\", \"/scripts\", \"/tmp\"]\n"
        );
    }

    #[test]
    fn an_existing_config_with_no_roots_line_gets_one_prepended() {
        let home = tempfile::tempdir().unwrap();
        let cfg = codex_cfg(home.path());
        fs::create_dir_all(cfg.parent().unwrap()).unwrap();
        fs::write(&cfg, "model = \"o3\"\n").unwrap();

        let outcome =
            codex_planning_permissions("/scripts", "/plans", "/tmp", home.path()).unwrap();
        assert!(matches!(outcome, CodexOutcome::Prepended));

        let content = fs::read_to_string(&cfg).unwrap();
        assert!(content.starts_with("sandbox_workspace_write.writable_roots ="));
        assert!(content.ends_with("model = \"o3\"\n"));
    }

    #[test]
    fn a_second_codex_grant_adds_only_the_missing_path() {
        let home = tempfile::tempdir().unwrap();
        codex_planning_permissions("/scripts", "/plans", "/tmp", home.path()).unwrap();

        let outcome = codex_worktrees_permissions("/wt", home.path()).unwrap();
        let added = match outcome {
            CodexOutcome::Appended(paths) => paths,
            _ => panic!("expected Appended"),
        };
        assert_eq!(added, vec!["/wt".to_string()]);

        let content = fs::read_to_string(codex_cfg(home.path())).unwrap();
        assert!(content.contains("\"/plans\", \"/scripts\", \"/tmp\", \"/wt\""));
    }

    #[test]
    fn a_third_codex_grant_with_nothing_new_reports_already_present() {
        let home = tempfile::tempdir().unwrap();
        codex_planning_permissions("/scripts", "/plans", "/tmp", home.path()).unwrap();

        let outcome =
            codex_planning_permissions("/scripts", "/plans", "/tmp", home.path()).unwrap();
        assert!(matches!(outcome, CodexOutcome::AlreadyPresent));
    }

    #[test]
    fn a_multiline_roots_array_is_refused_rather_than_guessed_at() {
        let home = tempfile::tempdir().unwrap();
        let cfg = codex_cfg(home.path());
        fs::create_dir_all(cfg.parent().unwrap()).unwrap();
        fs::write(
            &cfg,
            "sandbox_workspace_write.writable_roots = [\n  \"/existing\",\n]\n",
        )
        .unwrap();
        let before = fs::read_to_string(&cfg).unwrap();

        let outcome =
            codex_planning_permissions("/scripts", "/plans", "/tmp", home.path()).unwrap();
        assert!(matches!(outcome, CodexOutcome::NotSingleLineArray));
        assert_eq!(fs::read_to_string(&cfg).unwrap(), before);
    }

    #[test]
    fn a_file_with_no_trailing_newline_keeps_it_that_way() {
        let home = tempfile::tempdir().unwrap();
        let cfg = codex_cfg(home.path());
        fs::create_dir_all(cfg.parent().unwrap()).unwrap();
        fs::write(&cfg, "model = \"o3\"").unwrap();

        codex_planning_permissions("/scripts", "/plans", "/tmp", home.path()).unwrap();

        let content = fs::read_to_string(&cfg).unwrap();
        assert!(!content.ends_with('\n'));
    }
}
