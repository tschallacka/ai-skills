// MODE: DEV
// PACKAGE: PROD
//! Where the chat bridge queues the interrupts it could not push, and what
//! reads them: the bridge (`chat-mcp`) writes, the Claude Code PreToolUse hook
//! empties, and the spool watcher (`chat-spool-watch`) only looks. They share one
//! directory layout, so it lives here rather than in any of them:
//!
//! ```text
//! <state dir>/interrupts/<session id>/<identity>.log   one line per notice
//! <state dir>/interrupts/<session id>/.watcher         heartbeat of a live watcher
//! ```
//!
//! The session id is the harness's own (Claude Code's `CLAUDE_CODE_SESSION_ID`),
//! which both the bridge and the hook are handed, so neither needs the other's
//! identity logic.

use std::path::{Path, PathBuf};

/// The heartbeat a running watcher touches, so anything can tell one is armed.
/// It does not end in `.log`, so the hook, which reads `*.log`, never takes it.
pub const HEARTBEAT: &str = ".watcher";

/// Marks that this identity has hook-only interrupts configured (at least one
/// enabled, unexpired rule or a timer, with `delivery: hook`) -- the PreToolUse
/// hook's own signal that a `chat-spool-watch` heartbeat going stale means an
/// idle agent has nothing left watching for it, worth a reminder to re-arm.
/// Absent when there is nothing configured, or when delivery is `push` or
/// `both`, since a channel push already covers an idle agent then.
pub const ACTIVE: &str = ".active";

/// A name that cannot leave its directory: letters, digits, `-` and `_` only,
/// at most 96 characters. The hook applies the same rule in shell (`tr`).
pub fn safe_name(text: &str) -> String {
    text.chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .take(96)
        .collect()
}

/// The spool directory for one harness session.
pub fn dir(state_dir: &Path, session_id: &str) -> PathBuf {
    state_dir.join("interrupts").join(safe_name(session_id))
}

/// The chat state directory as the client resolves it: `$AI_CHAT_HOME`, else the
/// XDG config home's `tsch-ai-skills/chat`, else `~/.config/...`. (The client
/// also takes `--state`; a spool reader has no such flag and uses this.)
pub fn default_state_dir() -> PathBuf {
    if let Some(home) = std::env::var("AI_CHAT_HOME").ok().filter(|v| !v.is_empty()) {
        return PathBuf::from(home);
    }
    let config = match std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|v| !v.is_empty())
    {
        Some(v) => PathBuf::from(v),
        None => PathBuf::from(
            std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .unwrap_or_else(|_| ".".to_string()),
        )
        .join(".config"),
    };
    config.join("tsch-ai-skills").join("chat")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_name_cannot_leave_its_directory() {
        assert_eq!(safe_name("../../etc"), "______etc");
        assert_eq!(safe_name("h-4474b93c059ebd63"), "h-4474b93c059ebd63");
        assert_eq!(safe_name(&"x".repeat(500)).len(), 96);
    }

    #[test]
    fn the_directory_is_under_interrupts_and_named_by_the_session() {
        let dir = dir(Path::new("/state"), "724a0b94-7212");
        assert_eq!(dir, Path::new("/state/interrupts/724a0b94-7212"));
    }

    #[test]
    fn the_heartbeat_is_not_a_log_file() {
        assert!(!HEARTBEAT.ends_with(".log"));
    }

    #[test]
    fn the_active_marker_is_not_a_log_file_and_differs_from_the_heartbeat() {
        assert!(!ACTIVE.ends_with(".log"));
        assert_ne!(ACTIVE, HEARTBEAT);
    }
}
