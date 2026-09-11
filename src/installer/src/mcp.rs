// MODE: DEV
// PACKAGE: PROD
//! Registers an mcp-mode skill's adapter binary with an agent's own CLI, and
//! takes the registration away again once nothing points at it anymore --
//! ported from installer/src/72-mcp-registration.sh.
//!
//! Each agent's own CLI is preferred over hand-editing its configuration
//! (`claude mcp add`, `codex mcp add`), because the CLI owns the format;
//! opencode has no removal subcommand, so that direction edits its config
//! JSON directly with serde_json, reusing permissions.rs's atomic-write and
//! config-resolution helpers. Removal only ever touches an entry whose
//! command points inside the directory this install owns -- a hand-made
//! entry of the same name pointing elsewhere is left alone.
//!
//! Unlike claude_register/codex_register, which shell out and are therefore
//! NOT exercised by an automated test that could touch a real, live agent
//! configuration, the file-reading (`mcp_entry_is_ours`) and file-writing
//! (opencode's fallback) paths are fully unit tested. This mirrors the
//! ci-failures skill's own documented split (gh exercised for real, glab
//! only against a stub) rather than hiding the gap.

use crate::backup;
use crate::permissions;
use serde_json::Value;
use std::fs;
use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

pub enum RegisterOutcome {
    Registered,
    /// The agent's CLI is not on PATH, or invoking it failed; the caller
    /// prints manual instructions.
    Manual,
}

fn on_path(bin: &str) -> bool {
    let Ok(path_var) = std::env::var("PATH") else {
        return false;
    };
    std::env::split_paths(&path_var).any(|dir| is_executable(&dir.join(bin)))
}

fn is_executable(candidate: &Path) -> bool {
    if !candidate.is_file() {
        return false;
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(candidate)
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
    }
    #[cfg(not(unix))]
    {
        true
    }
}

fn claude_mcp_command(home: &Path, name: &str) -> Option<String> {
    let raw = fs::read_to_string(home.join(".claude.json")).ok()?;
    let doc: Value = serde_json::from_str(&raw).ok()?;
    doc.get("mcpServers")?
        .get(name)?
        .get("command")?
        .as_str()
        .map(String::from)
}

/// `[mcp_servers.NAME]`'s own `command` key, read the same way install.sh's
/// awk one-liner does: stop at the next `[table]` header, and only look at
/// lines whose first whitespace-separated token is exactly `command`.
fn codex_mcp_command(content: &str, name: &str) -> Option<String> {
    let want = format!("[mcp_servers.{name}]");
    let mut inside = false;
    for line in content.lines() {
        if line == want {
            inside = true;
            continue;
        }
        if !inside {
            continue;
        }
        if line.starts_with('[') {
            break;
        }
        if line.split_whitespace().next() != Some("command") {
            continue;
        }
        let value = line.split_once('=')?.1.trim();
        return Some(value.trim_matches('"').to_string());
    }
    None
}

fn opencode_mcp_command(home: &Path, name: &str) -> Option<String> {
    let cfg = permissions::opencode_configfile(home);
    let raw = fs::read_to_string(cfg).ok()?;
    let doc: Value = serde_json::from_str(&raw).ok()?;
    doc.get("mcp")?
        .get(name)?
        .get("command")?
        .get(0)?
        .as_str()
        .map(String::from)
}

/// `mcp_entry_is_ours` in install.sh (installer/src/72-mcp-registration.sh)
/// reads `${CODEX_HOME:-$HOME/.codex}/config.toml` -- a different override
/// variable from the one its own permission-merge path uses
/// (`CODEX_CONFIGFILE`, `permissions::codex_configfile`). Two variables for
/// the same default file is install.sh's own inconsistency, not a slip in
/// this port: matching it means using CODEX_HOME here specifically, not
/// reusing `codex_configfile`.
fn codex_mcp_configfile(home: &Path) -> std::path::PathBuf {
    std::env::var("CODEX_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| home.join(".codex"))
        .join("config.toml")
}

/// Does an agent's registration for `name` point inside `dir`, the
/// directory this install owns? Only then is it this installer's to remove.
pub fn entry_is_ours(kind: &str, name: &str, dir: &Path, home: &Path) -> bool {
    let command = match kind {
        "claude" => claude_mcp_command(home, name),
        "codex" => fs::read_to_string(codex_mcp_configfile(home))
            .ok()
            .and_then(|content| codex_mcp_command(&content, name)),
        "opencode" => opencode_mcp_command(home, name),
        _ => None,
    };
    match command {
        Some(cmd) => cmd.starts_with(&format!("{}/", dir.display())),
        None => false,
    }
}

/// `claude mcp add` on a name that already exists reports it and exits 0
/// WITHOUT updating the command, so an upgrade that moved the binary would
/// keep the old path; removing first makes the add the step that decides.
fn claude_register(name: &str, path: &str) -> RegisterOutcome {
    if !on_path("claude") {
        return RegisterOutcome::Manual;
    }
    let _ = Command::new("claude")
        .args(["mcp", "remove", "-s", "user", name])
        .stdin(Stdio::null())
        .output();
    let added = Command::new("claude")
        .args(["mcp", "add", "-s", "user", "-t", "stdio", name, path])
        .stdin(Stdio::null())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if added {
        RegisterOutcome::Registered
    } else {
        RegisterOutcome::Manual
    }
}

fn codex_register(name: &str, path: &str) -> RegisterOutcome {
    if !on_path("codex") {
        return RegisterOutcome::Manual;
    }
    let added = Command::new("codex")
        .args(["mcp", "add", name, "--", path])
        .stdin(Stdio::null())
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if added {
        RegisterOutcome::Registered
    } else {
        RegisterOutcome::Manual
    }
}

/// stdin is closed because the same subcommand prompts when given no
/// command, and an installer must never stop on a prompt it did not intend.
fn opencode_register(name: &str, path: &str, home: &Path) -> io::Result<RegisterOutcome> {
    if on_path("opencode") {
        let added = Command::new("opencode")
            .args(["mcp", "add", name, "--", path])
            .stdin(Stdio::null())
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if added {
            return Ok(RegisterOutcome::Registered);
        }
    }
    opencode_write(name, path, home)
}

/// The fallback when opencode is not on PATH (or its own `add` failed): its
/// own shape, as it writes it -- type "local" and an argv array, no
/// `enabled` key.
fn opencode_write(name: &str, path: &str, home: &Path) -> io::Result<RegisterOutcome> {
    let cfg = permissions::opencode_configfile(home);
    if !cfg.is_file() {
        return Ok(RegisterOutcome::Manual);
    }
    backup::backup_file(&cfg)?;
    let raw = fs::read_to_string(&cfg)?;
    let mut doc = permissions::as_object(serde_json::from_str(&raw).ok());
    let mut mcp = permissions::as_object(doc.get("mcp").cloned());
    let mut entry = serde_json::Map::new();
    entry.insert("type".to_string(), Value::String("local".to_string()));
    entry.insert(
        "command".to_string(),
        Value::Array(vec![Value::String(path.to_string())]),
    );
    mcp.insert(name.to_string(), Value::Object(entry));
    doc.insert("mcp".to_string(), Value::Object(mcp));
    permissions::write_preserving_mode(&cfg, &serde_json::to_string_pretty(&Value::Object(doc))?)?;
    Ok(RegisterOutcome::Registered)
}

fn opencode_unregister(name: &str, home: &Path) -> io::Result<bool> {
    let cfg = permissions::opencode_configfile(home);
    if !cfg.is_file() {
        return Ok(false);
    }
    let raw = fs::read_to_string(&cfg)?;
    let mut doc = permissions::as_object(serde_json::from_str(&raw).ok());
    let Some(Value::Object(mut mcp)) = doc.get("mcp").cloned() else {
        return Ok(false);
    };
    if mcp.remove(name).is_none() {
        return Ok(false);
    }
    backup::backup_file(&cfg)?;
    doc.insert("mcp".to_string(), Value::Object(mcp));
    permissions::write_preserving_mode(&cfg, &serde_json::to_string_pretty(&Value::Object(doc))?)?;
    Ok(true)
}

pub fn register_for_kind(
    kind: &str,
    name: &str,
    path: &str,
    home: &Path,
) -> io::Result<RegisterOutcome> {
    match kind {
        "claude" => Ok(claude_register(name, path)),
        "codex" => Ok(codex_register(name, path)),
        "opencode" => opencode_register(name, path, home),
        _ => Ok(RegisterOutcome::Manual),
    }
}

/// Removes a registration only when `entry_is_ours` says this install put
/// it there; a hand-made entry of the same name pointing elsewhere is left
/// alone. Returns whether anything was actually removed.
pub fn unregister_for_kind(kind: &str, name: &str, dir: &Path, home: &Path) -> io::Result<bool> {
    if !entry_is_ours(kind, name, dir, home) {
        return Ok(false);
    }
    match kind {
        "claude" => Ok(Command::new("claude")
            .args(["mcp", "remove", "-s", "user", name])
            .stdin(Stdio::null())
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)),
        "codex" => Ok(Command::new("codex")
            .args(["mcp", "remove", name])
            .stdin(Stdio::null())
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)),
        "opencode" => opencode_unregister(name, home),
        _ => Ok(false),
    }
}

pub fn manual_instructions(kind: &str, name: &str, path: &str) -> Vec<String> {
    let mut lines = vec![format!("{kind}: register the MCP server by hand:")];
    lines.push(match kind {
        "claude" => format!("    claude mcp add -s user -t stdio {name} {path}"),
        "codex" => format!("    codex mcp add {name} -- {path}"),
        "opencode" => format!("    opencode mcp add {name} -- {path}"),
        _ => format!("    run {path} as a stdio MCP server named {name}"),
    });
    lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // codex_mcp_configfile reads the process-global CODEX_HOME; every test
    // that overrides it takes this lock first, same reasoning as
    // plan_migration.rs's own ENV_LOCK.
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn write(path: &Path, content: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    #[test]
    fn codex_mcp_command_reads_the_named_table_and_stops_at_the_next_one() {
        let toml = "\
[mcp_servers.foo]
command = \"/opt/foo/bin\"

[mcp_servers.bar]
command = \"/opt/bar/bin\"
";
        assert_eq!(
            codex_mcp_command(toml, "foo"),
            Some("/opt/foo/bin".to_string())
        );
        assert_eq!(
            codex_mcp_command(toml, "bar"),
            Some("/opt/bar/bin".to_string())
        );
        assert_eq!(codex_mcp_command(toml, "missing"), None);
    }

    #[test]
    fn entry_is_ours_for_codex_honors_codex_home_not_codex_configfile() {
        let _guard = ENV_LOCK.lock().unwrap();
        let home = tempfile::tempdir().unwrap();
        let codex_home = tempfile::tempdir().unwrap();
        write(
            &codex_home.path().join("config.toml"),
            "[mcp_servers.todo]\ncommand = \"/skills/todo/bin/adapter\"\n",
        );
        std::env::set_var("CODEX_HOME", codex_home.path());
        let is_ours = entry_is_ours("codex", "todo", Path::new("/skills/todo"), home.path());
        std::env::remove_var("CODEX_HOME");
        assert!(is_ours);
    }

    #[test]
    fn entry_is_ours_for_claude_matches_only_a_command_inside_the_directory() {
        let home = tempfile::tempdir().unwrap();
        write(
            &home.path().join(".claude.json"),
            r#"{"mcpServers":{"ai-text-editor":{"command":"/skills/ai-text-editor/bin/adapter"}}}"#,
        );
        assert!(entry_is_ours(
            "claude",
            "ai-text-editor",
            Path::new("/skills/ai-text-editor"),
            home.path()
        ));
        assert!(!entry_is_ours(
            "claude",
            "ai-text-editor",
            Path::new("/somewhere/else"),
            home.path()
        ));
        assert!(!entry_is_ours(
            "claude",
            "no-such-entry",
            Path::new("/skills/ai-text-editor"),
            home.path()
        ));
    }

    #[test]
    fn entry_is_ours_for_opencode_reads_the_command_array() {
        let home = tempfile::tempdir().unwrap();
        write(
            &home.path().join(".config/opencode/opencode.json"),
            r#"{"mcp":{"ai-text-editor":{"type":"local","command":["/skills/ai-text-editor/bin/adapter"]}}}"#,
        );
        assert!(entry_is_ours(
            "opencode",
            "ai-text-editor",
            Path::new("/skills/ai-text-editor"),
            home.path()
        ));
    }

    #[test]
    fn opencode_write_creates_the_local_command_shape() {
        let home = tempfile::tempdir().unwrap();
        write(&home.path().join(".config/opencode/opencode.json"), "{}");

        let outcome = opencode_write(
            "ai-text-editor",
            "/skills/ai-text-editor/bin/adapter",
            home.path(),
        )
        .unwrap();
        assert!(matches!(outcome, RegisterOutcome::Registered));

        let cfg = home.path().join(".config/opencode/opencode.json");
        let doc: Value = serde_json::from_str(&fs::read_to_string(cfg).unwrap()).unwrap();
        assert_eq!(doc["mcp"]["ai-text-editor"]["type"], "local");
        assert_eq!(
            doc["mcp"]["ai-text-editor"]["command"][0],
            "/skills/ai-text-editor/bin/adapter"
        );
    }

    #[test]
    fn opencode_write_with_no_config_file_reports_manual() {
        let home = tempfile::tempdir().unwrap();
        let outcome = opencode_write("name", "/path", home.path()).unwrap();
        assert!(matches!(outcome, RegisterOutcome::Manual));
    }

    #[test]
    fn opencode_unregister_removes_only_the_named_entry() {
        let home = tempfile::tempdir().unwrap();
        let cfg = home.path().join(".config/opencode/opencode.json");
        write(
            &cfg,
            r#"{"mcp":{"keep-me":{"type":"local","command":["/a"]},"ai-text-editor":{"type":"local","command":["/b"]}}}"#,
        );

        let removed = opencode_unregister("ai-text-editor", home.path()).unwrap();
        assert!(removed);

        let doc: Value = serde_json::from_str(&fs::read_to_string(&cfg).unwrap()).unwrap();
        assert!(doc["mcp"].get("ai-text-editor").is_none());
        assert!(doc["mcp"].get("keep-me").is_some());
    }

    #[test]
    fn opencode_unregister_on_a_name_not_present_reports_nothing_removed() {
        let home = tempfile::tempdir().unwrap();
        write(&home.path().join(".config/opencode/opencode.json"), "{}");
        let removed = opencode_unregister("no-such-name", home.path()).unwrap();
        assert!(!removed);
    }

    #[test]
    fn manual_instructions_name_the_right_command_per_agent() {
        let claude = manual_instructions("claude", "n", "/p");
        assert!(claude[1].contains("claude mcp add -s user -t stdio n /p"));
        let codex = manual_instructions("codex", "n", "/p");
        assert!(codex[1].contains("codex mcp add n -- /p"));
    }
}
