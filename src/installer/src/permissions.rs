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

/// `${XDG_CONFIG_HOME:-$HOME/.config}/tsch-ai-worktrees` -- ported from
/// `worktrees_permission_step` in installer/src/70-permissions.sh. Unlike
/// `plan_migration::default_root`'s plan root, there is no dedicated
/// override variable for this one in install.sh either.
pub fn default_worktrees_root(home: &Path) -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".config"));
    base.join("tsch-ai-worktrees")
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

/// interactive-shell's own two shipped binaries (never
/// `interactive-shell-fixture`, a maintainer-only build with no use on a
/// target machine -- see interactive-shell/binaries.tsv). Named explicitly
/// rather than granted as a directory glob: since T72, `bins` names the
/// shared XDG bin directory every skill's compiled binary now lives in
/// together, so a directory-scoped grant there would silently authorize
/// no-prompt execution of every OTHER skill's binary too, not just these
/// two.
const INTERACTIVE_SHELL_BINARIES: &[&str] = &["interactive-shell", "interactive-shell-input"];

fn interactive_shell_entries(bins: &str) -> Vec<String> {
    let bins = strip_trailing_slashes(bins);
    INTERACTIVE_SHELL_BINARIES
        .iter()
        .map(|name| format!("Bash({bins}/{name}:*)"))
        .collect()
}

/// A denied Bash call does not read as "ask for permission" to an agent --
/// it reads as "this tool does not work", after which the agent falls back
/// to a headless invocation that cannot observe the program at all. `bins`
/// is the directory interactive-shell's binaries are found in -- the shared
/// XDG bin directory (`shared_bin::shared_bin_dir`) for the two call sites
/// that resolve it automatically; a caller of the standalone
/// `grant-permissions --bins` CLI names it explicitly instead.
pub fn claude_interactive_shell_permissions(
    bins: &str,
    home: &Path,
) -> io::Result<PermissionOutcome> {
    let cfg = claude_settings_path(home);
    if !cfg.is_file() {
        return Ok(PermissionOutcome::NoConfigFile);
    }
    merge_allow_entries(&cfg, &interactive_shell_entries(bins))
}

pub enum EnvSettingOutcome {
    NoConfigFile,
    AlreadySet,
    Set,
}

/// One `env.KEY` merged into Claude's settings.json -- same write discipline
/// as the permission editors above (backup, defensive read, atomic rename).
pub fn claude_env_setting(key: &str, value: &str, home: &Path) -> io::Result<EnvSettingOutcome> {
    let cfg = claude_settings_path(home);
    if !cfg.is_file() {
        return Ok(EnvSettingOutcome::NoConfigFile);
    }
    backup::backup_file(&cfg)?;
    let raw = fs::read_to_string(&cfg)?;
    let mut doc = as_object(serde_json::from_str(&raw).ok());
    let mut env = as_object(doc.get("env").cloned());
    let already_set = env.get(key).and_then(Value::as_str) == Some(value);
    env.insert(key.to_string(), Value::String(value.to_string()));
    doc.insert("env".to_string(), Value::Object(env));
    write_preserving_mode(&cfg, &serde_json::to_string_pretty(&Value::Object(doc))?)?;
    Ok(if already_set {
        EnvSettingOutcome::AlreadySet
    } else {
        EnvSettingOutcome::Set
    })
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

pub enum PermissionRemovalOutcome {
    NoConfigFile,
    /// None of `entries` was present; nothing was touched.
    NothingToRemove,
    Removed(Vec<String>),
}

/// The exact inverse of `merge_allow_entries`: deletes `entries` from
/// `permissions.allow`, leaving every other entry (including one a user
/// added independently) untouched.
fn remove_allow_entries(cfg: &Path, entries: &[String]) -> io::Result<PermissionRemovalOutcome> {
    if !cfg.is_file() {
        return Ok(PermissionRemovalOutcome::NoConfigFile);
    }
    backup::backup_file(cfg)?;

    let raw = fs::read_to_string(cfg)?;
    let mut doc = as_object(serde_json::from_str(&raw).ok());
    let mut permissions = as_object(doc.get("permissions").cloned());
    let allow = allow_array(&permissions);

    let to_remove: Vec<Value> = entries.iter().cloned().map(Value::String).collect();
    let removed: Vec<String> = entries
        .iter()
        .filter(|e| allow.contains(&Value::String((*e).clone())))
        .cloned()
        .collect();
    if removed.is_empty() {
        return Ok(PermissionRemovalOutcome::NothingToRemove);
    }

    let new_allow: Vec<Value> = allow
        .into_iter()
        .filter(|v| !to_remove.contains(v))
        .collect();
    permissions.insert("allow".to_string(), Value::Array(new_allow));
    doc.insert("permissions".to_string(), Value::Object(permissions));

    write_preserving_mode(cfg, &serde_json::to_string_pretty(&Value::Object(doc))?)?;
    Ok(PermissionRemovalOutcome::Removed(removed))
}

pub fn claude_planning_permissions_remove(
    scripts: &str,
    plans: &str,
    tmp: &str,
    home: &Path,
) -> io::Result<PermissionRemovalOutcome> {
    let cfg = claude_settings_path(home);
    let entries = planning_entries(
        strip_trailing_slashes(scripts),
        strip_trailing_slashes(plans),
        strip_trailing_slashes(tmp),
    );
    remove_allow_entries(&cfg, &entries)
}

pub fn claude_worktrees_permissions_remove(
    worktrees: &str,
    home: &Path,
) -> io::Result<PermissionRemovalOutcome> {
    let cfg = claude_settings_path(home);
    let worktrees = strip_trailing_slashes(worktrees);
    let entries = vec![
        format!("Read({worktrees}/**)"),
        format!("Edit({worktrees}/**)"),
        format!("Bash({worktrees}/**:*)"),
    ];
    remove_allow_entries(&cfg, &entries)
}

pub fn claude_interactive_shell_permissions_remove(
    bins: &str,
    home: &Path,
) -> io::Result<PermissionRemovalOutcome> {
    let cfg = claude_settings_path(home);
    remove_allow_entries(&cfg, &interactive_shell_entries(bins))
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
pub(crate) fn opencode_prepare_config(cfg: &Path) -> io::Result<Option<()>> {
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

/// The exact inverse of `opencode_merge_permission`: for each (tool, pattern)
/// in `wanted`, deletes the pattern key from that tool's rule map ONLY when
/// its current value is still exactly `"allow"` -- a value the user changed
/// to `"deny"`/`"ask"` after installation is left alone, the same ownership-
/// safety principle `mcp::entry_is_ours` already applies to MCP entries.
fn opencode_unmerge_permission(cfg: &Path, wanted: &[WantedRule]) -> io::Result<Vec<String>> {
    if !cfg.is_file() {
        return Ok(Vec::new());
    }
    let raw = fs::read_to_string(cfg)?;
    let doc = as_object(serde_json::from_str(&raw).ok());
    let tools: Vec<&str> = wanted.iter().map(|(tool, _)| *tool).collect();
    let base = base_permission(doc.get("permission"), &tools);

    let mut removed = Vec::new();
    let mut perm = base.clone();
    for (tool, patterns) in wanted {
        let mut rule = rules_of(base.get(*tool));
        for pattern in *patterns {
            if rule.get(pattern).and_then(Value::as_str) == Some("allow") {
                rule.remove(pattern);
                removed.push(format!("{tool}: {pattern}"));
            }
        }
        perm.insert(tool.to_string(), Value::Object(rule));
    }
    if removed.is_empty() {
        return Ok(removed);
    }

    backup::backup_file(cfg)?;
    let mut doc = doc;
    doc.insert("permission".to_string(), Value::Object(perm));
    write_preserving_mode(cfg, &serde_json::to_string_pretty(&Value::Object(doc))?)?;
    Ok(removed)
}

pub fn opencode_planning_permissions_remove(
    scripts: &str,
    plans: &str,
    tmp: &str,
    home: &Path,
) -> io::Result<Vec<String>> {
    let cfg = opencode_configfile(home);
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
    opencode_unmerge_permission(
        &cfg,
        &[
            ("read", &read),
            ("edit", &edit),
            ("bash", &bash),
            ("external_directory", &external_directory),
        ],
    )
}

pub fn opencode_worktrees_permissions_remove(
    worktrees: &str,
    home: &Path,
) -> io::Result<Vec<String>> {
    let cfg = opencode_configfile(home);
    let pattern = vec![format!("{}/**", strip_trailing_slashes(worktrees))];
    opencode_unmerge_permission(
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

/// The exact inverse of `codex_merge_writable_roots`: strips only the quoted
/// paths in `unwanted` from the single-line bracketed array, leaving `[]`
/// rather than deleting the line when nothing remains.
fn codex_unmerge_writable_roots(cfg: &Path, unwanted: &[String]) -> io::Result<Vec<String>> {
    if !cfg.is_file() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(cfg)?;
    let Some(line_index) = writable_roots_line(&content) else {
        return Ok(Vec::new());
    };
    let mut lines: Vec<String> = content.lines().map(str::to_string).collect();
    let line = lines[line_index].clone();
    let Some((before, rest)) = line.split_once('[') else {
        return Ok(Vec::new());
    };
    let Some((inside, after)) = rest.split_once(']') else {
        return Ok(Vec::new());
    };
    let kept: Vec<&str> = inside
        .split(',')
        .map(str::trim)
        .filter(|entry| !entry.is_empty())
        .collect();
    let removed: Vec<String> = unwanted
        .iter()
        .filter(|p| kept.contains(&format!("\"{p}\"").as_str()))
        .cloned()
        .collect();
    if removed.is_empty() {
        return Ok(removed);
    }

    backup::backup_file(cfg)?;
    let remaining: Vec<&str> = kept
        .into_iter()
        .filter(|entry| !removed.iter().any(|p| *entry == format!("\"{p}\"")))
        .collect();
    lines[line_index] = format!("{before}[{}]{after}", remaining.join(", "));
    write_preserving_mode(cfg, &rejoin_preserving_trailing_newline(&content, lines))?;
    Ok(removed)
}

pub fn codex_planning_permissions_remove(
    scripts: &str,
    plans: &str,
    tmp: &str,
    home: &Path,
) -> io::Result<Vec<String>> {
    let cfg = codex_configfile(home);
    let unwanted = vec![
        strip_trailing_slashes(plans).to_string(),
        strip_trailing_slashes(scripts).to_string(),
        strip_trailing_slashes(tmp).to_string(),
    ];
    codex_unmerge_writable_roots(&cfg, &unwanted)
}

pub fn codex_worktrees_permissions_remove(worktrees: &str, home: &Path) -> io::Result<Vec<String>> {
    let cfg = codex_configfile(home);
    let unwanted = vec![strip_trailing_slashes(worktrees).to_string()];
    codex_unmerge_writable_roots(&cfg, &unwanted)
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

    #[test]
    fn claude_removal_deletes_only_the_added_entries() {
        let home = tempfile::tempdir().unwrap();
        settings_at(home.path(), r#"{"permissions":{"allow":["Bash(ls:*)"]}}"#);
        claude_worktrees_permissions("/worktrees", home.path()).unwrap();

        let outcome = claude_worktrees_permissions_remove("/worktrees", home.path()).unwrap();
        let removed = match outcome {
            PermissionRemovalOutcome::Removed(entries) => entries,
            _ => panic!("expected Removed"),
        };
        assert_eq!(removed.len(), 3);

        let doc: Value =
            serde_json::from_str(&fs::read_to_string(claude_settings_path(home.path())).unwrap())
                .unwrap();
        let allow: Vec<&str> = doc["permissions"]["allow"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        assert_eq!(allow, vec!["Bash(ls:*)"]);
    }

    #[test]
    fn claude_removal_of_an_absent_grant_reports_nothing_to_remove() {
        let home = tempfile::tempdir().unwrap();
        settings_at(home.path(), "{}");
        let outcome = claude_worktrees_permissions_remove("/worktrees", home.path()).unwrap();
        assert!(matches!(outcome, PermissionRemovalOutcome::NothingToRemove));
    }

    #[test]
    fn opencode_removal_deletes_only_a_pattern_still_set_to_allow() {
        let home = tempfile::tempdir().unwrap();
        opencode_worktrees_permissions("/worktrees", home.path()).unwrap();
        let cfg = opencode_cfg(home.path());
        let mut doc: Value = serde_json::from_str(&fs::read_to_string(&cfg).unwrap()).unwrap();
        doc["permission"]["bash"]["/worktrees/**"] = Value::String("deny".to_string());
        fs::write(&cfg, serde_json::to_string_pretty(&doc).unwrap()).unwrap();

        let removed = opencode_worktrees_permissions_remove("/worktrees", home.path()).unwrap();

        assert!(!removed.contains(&"bash: /worktrees/**".to_string()));
        assert!(removed.contains(&"read: /worktrees/**".to_string()));
        let doc: Value = serde_json::from_str(&fs::read_to_string(&cfg).unwrap()).unwrap();
        assert_eq!(doc["permission"]["bash"]["/worktrees/**"], "deny");
        assert!(doc["permission"]["read"].get("/worktrees/**").is_none());
    }

    #[test]
    fn opencode_removal_of_an_absent_grant_reports_nothing() {
        let home = tempfile::tempdir().unwrap();
        let removed = opencode_worktrees_permissions_remove("/worktrees", home.path()).unwrap();
        assert!(removed.is_empty());
    }

    #[test]
    fn codex_removal_strips_only_the_named_paths() {
        let home = tempfile::tempdir().unwrap();
        codex_planning_permissions("/scripts", "/plans", "/tmp", home.path()).unwrap();

        let removed = codex_worktrees_permissions_remove("/worktrees", home.path()).unwrap();
        assert!(
            removed.is_empty(),
            "worktrees removal must not touch planning's own paths"
        );

        let removed =
            codex_planning_permissions_remove("/scripts", "/plans", "/tmp", home.path()).unwrap();
        assert_eq!(removed.len(), 3);

        let content = fs::read_to_string(codex_cfg(home.path())).unwrap();
        assert_eq!(content, "sandbox_workspace_write.writable_roots = []\n");
    }

    #[test]
    fn codex_removal_leaves_an_untouched_path_in_place() {
        let home = tempfile::tempdir().unwrap();
        codex_worktrees_permissions("/worktrees", home.path()).unwrap();
        codex_planning_permissions("/scripts", "/plans", "/tmp", home.path()).unwrap();

        codex_worktrees_permissions_remove("/worktrees", home.path()).unwrap();

        let content = fs::read_to_string(codex_cfg(home.path())).unwrap();
        assert!(!content.contains("\"/worktrees\""));
        assert!(content.contains("\"/plans\""));
    }
}
