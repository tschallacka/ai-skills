// MODE: DEV
// PACKAGE: PROD
//! Resolves which agent an invocation belongs to, as a precedence ladder,
//! ported out of the chat skill (`chat-client-rs`) so a second tool
//! (`ai-text-editor`) can key its own per-agent session state the same way,
//! without either depending on the other.
//!
//! Deliberately absent: anything read out of the process tree. `pid`, `ppid`
//! and `getsid` were all measured to change between two invocations by the
//! same agent (a runner such as `timeout` or `env`, or the harness re-execing,
//! gives a fresh pid every call), which would mint a new key per call and lose
//! whatever the caller keys off it. Inside codex's sandbox they are worse than
//! unstable: they are pinned at 3/2/1 for every session on the machine, so
//! they are stable and identical, which would merge every codex agent into
//! one.

/// Where the resolved key came from, so a caller can tell a shared key from
/// its own and say which rung of the ladder decided.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeySource {
    /// The caller's own explicit id, or the env var it names for "told
    /// directly" (e.g. `CHAT_SESSION_ID`, `TSCH_AI_EDITOR_AGENT`).
    Explicit,
    /// A session id the coding harness itself exports.
    Harness,
    /// The git worktree root: a zero-config default for agents that each
    /// work in their own checkout of one project.
    Worktree,
    /// Nothing distinguished this agent, so it shares one key.
    Shared,
}

impl KeySource {
    pub fn as_str(self) -> &'static str {
        match self {
            KeySource::Explicit => "explicit",
            KeySource::Harness => "harness",
            KeySource::Worktree => "worktree",
            KeySource::Shared => "shared",
        }
    }
}

/// Harness-exported identity variables, most specific first. Each was
/// measured on the maintainer's machine to be identical across repeated
/// invocations of one agent (including through `env`, `timeout` and a shell
/// function wrapper) and to differ between genuinely different agents:
///
/// - `CLAUDE_CODE_SESSION_ID` -- Claude Code, one per session and per subagent.
/// - `CODEX_SESSION_ID` -- codex; `CODEX_THREAD_ID` was measured equal to it,
///   so it adds nothing.
/// - `OPENCODE_PID` -- opencode exports no session id, only the pid of the
///   opencode process. That is instance granularity, not session granularity:
///   several sessions inside one opencode process share it, and a recycled
///   pid can adopt a dead instance's cursors. It is still the only thing
///   opencode offers, and it is a value the harness exports rather than
///   something read back out of the process tree, so it does not move per
///   invocation.
///
/// Every variable that is set contributes to the key, rather than the first
/// one winning. Harnesses nest: a codex launched from a Claude Code agent
/// inherits that agent's `CLAUDE_CODE_SESSION_ID` unchanged and adds its own
/// `CODEX_SESSION_ID` (measured). Taking only the first match would give the
/// inner codex the outer agent's session; combining them keeps the two apart
/// whichever way round they are nested.
pub const HARNESS_ID_VARS: [&str; 3] =
    ["CLAUDE_CODE_SESSION_ID", "CODEX_SESSION_ID", "OPENCODE_PID"];

/// FNV-1a, 64-bit. Not a cryptographic hash and does not need to be: it only
/// turns an identity string into a short, stable, filename-safe key.
pub fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Reduce a caller-chosen id to something safe to use as a filename or a
/// registry key, keeping it readable so a listing still says whose is whose.
pub fn safe_key(raw: &str) -> String {
    let mut out = String::new();
    for c in raw.chars() {
        if c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == '_' {
            out.push(c);
        } else {
            out.push('_');
        }
        if out.len() >= 64 {
            break;
        }
    }
    // "." and ".." would name a directory rather than a key.
    if out.is_empty() || out.chars().all(|c| c == '.') {
        return String::new();
    }
    out
}

/// Resolve which agent this invocation belongs to. Pure: the environment and
/// the worktree root are handed in, so each rung can be tested without an
/// actual agent, an actual harness, or an actual repository.
///
/// `explicit_env_var` names the env var this caller treats as "told
/// directly" alongside its own `explicit` argument (chat uses
/// `CHAT_SESSION_ID`; a different tool names its own) — both are safe_key'd
/// and either can win rung 1, `explicit` taking priority.
pub fn resolve_session_key(
    explicit: Option<&str>,
    explicit_env_var: &str,
    env: &dyn Fn(&str) -> Option<String>,
    worktree_root: Option<&str>,
) -> (String, KeySource) {
    // 1. What the caller asked for by name always wins; no inference.
    let chosen = explicit
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| env(explicit_env_var).filter(|s| !s.is_empty()));
    if let Some(id) = chosen {
        let key = safe_key(&id);
        if !key.is_empty() {
            return (key, KeySource::Explicit);
        }
    }

    // 2. Whatever identity the harness already knows about itself.
    let mut material = String::new();
    for name in HARNESS_ID_VARS.iter() {
        if let Some(v) = env(name).filter(|v| !v.is_empty()) {
            material.push_str(name);
            material.push('=');
            material.push_str(&v);
            material.push('\u{1f}');
        }
    }
    if !material.is_empty() {
        return (
            format!("h-{:016x}", fnv1a64(material.as_bytes())),
            KeySource::Harness,
        );
    }

    // 3. The worktree root. Agents on one project in separate worktrees are
    //    the case this ladder exists for, and the root is already unique per
    //    checkout on a machine, so the shared repository directory would add
    //    nothing to distinctness -- two checkouts of one repo have different
    //    roots, and sibling worktrees must NOT share a key.
    if let Some(root) = worktree_root.filter(|r| !r.is_empty()) {
        return (
            format!("w-{:016x}", fnv1a64(root.as_bytes())),
            KeySource::Worktree,
        );
    }

    // 4. Nothing to go on -- outside a repository, with no harness and no
    //    explicit id. One shared key, under a name that says so.
    ("shared".to_string(), KeySource::Shared)
}

/// The current worktree root, or `None` outside a git repository (or where
/// git is not installed, which must degrade to the next rung rather than
/// fail).
pub fn git_worktree_root() -> Option<String> {
    let out = std::process::Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let root = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if root.is_empty() {
        None
    } else {
        Some(root)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn env_of<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name: &str| {
            pairs
                .iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| v.to_string())
        }
    }

    #[test]
    fn explicit_flag_wins_over_everything() {
        let env = env_of(&[
            ("CHAT_SESSION_ID", "from-env"),
            ("CLAUDE_CODE_SESSION_ID", "claude-1"),
        ]);
        let (key, src) =
            resolve_session_key(Some("agent-b"), "CHAT_SESSION_ID", &env, Some("/repo"));
        assert_eq!(key, "agent-b");
        assert_eq!(src, KeySource::Explicit);
    }

    #[test]
    fn explicit_env_var_wins_over_inference() {
        let env = env_of(&[
            ("CHAT_SESSION_ID", "from-env"),
            ("CLAUDE_CODE_SESSION_ID", "claude-1"),
        ]);
        let (key, src) = resolve_session_key(None, "CHAT_SESSION_ID", &env, Some("/repo"));
        assert_eq!(key, "from-env");
        assert_eq!(src, KeySource::Explicit);
    }

    #[test]
    fn a_different_callers_explicit_env_var_name_is_honored() {
        let env = env_of(&[("TSCH_AI_EDITOR_AGENT", "agent-x")]);
        let (key, src) = resolve_session_key(None, "TSCH_AI_EDITOR_AGENT", &env, None);
        assert_eq!(key, "agent-x");
        assert_eq!(src, KeySource::Explicit);
    }

    #[test]
    fn explicit_id_is_reduced_to_a_safe_filename() {
        let env = env_of(&[]);
        let (key, src) =
            resolve_session_key(Some("../../etc/passwd"), "CHAT_SESSION_ID", &env, None);
        assert_eq!(src, KeySource::Explicit);
        assert!(!key.contains('/'), "key must not contain a path separator");
    }

    #[test]
    fn each_harness_session_id_gives_a_distinct_key() {
        let a = resolve_session_key(
            None,
            "CHAT_SESSION_ID",
            &env_of(&[("CLAUDE_CODE_SESSION_ID", "a")]),
            None,
        );
        let b = resolve_session_key(
            None,
            "CHAT_SESSION_ID",
            &env_of(&[("CLAUDE_CODE_SESSION_ID", "b")]),
            None,
        );
        assert_eq!(a.1, KeySource::Harness);
        assert_ne!(a.0, b.0, "two Claude Code sessions must not share a key");
    }

    #[test]
    fn a_harness_key_is_stable_for_the_same_ids() {
        let pairs = [("CODEX_SESSION_ID", "same"), ("OPENCODE_PID", "9")];
        let first = resolve_session_key(None, "CHAT_SESSION_ID", &env_of(&pairs), Some("/a"));
        let again = resolve_session_key(None, "CHAT_SESSION_ID", &env_of(&pairs), Some("/b"));
        assert_eq!(
            first.0, again.0,
            "the harness rung must not depend on the worktree"
        );
    }

    #[test]
    fn a_nested_harness_does_not_inherit_the_outer_agents_session() {
        let outer = resolve_session_key(
            None,
            "CHAT_SESSION_ID",
            &env_of(&[("CLAUDE_CODE_SESSION_ID", "a")]),
            None,
        );
        let inner = resolve_session_key(
            None,
            "CHAT_SESSION_ID",
            &env_of(&[("CLAUDE_CODE_SESSION_ID", "a"), ("CODEX_SESSION_ID", "z")]),
            None,
        );
        assert_ne!(
            outer.0, inner.0,
            "the inner codex must get its own session, not the outer agent's"
        );
    }

    #[test]
    fn the_worktree_rung_separates_worktrees_and_only_applies_without_a_harness() {
        let env = env_of(&[]);
        let a = resolve_session_key(
            None,
            "CHAT_SESSION_ID",
            &env,
            Some("/repo/.claude/worktrees/one"),
        );
        let b = resolve_session_key(
            None,
            "CHAT_SESSION_ID",
            &env,
            Some("/repo/.claude/worktrees/two"),
        );
        assert_eq!(a.1, KeySource::Worktree);
        assert_ne!(a.0, b.0, "sibling worktrees must not share a key");
        assert_eq!(
            a.0,
            resolve_session_key(
                None,
                "CHAT_SESSION_ID",
                &env,
                Some("/repo/.claude/worktrees/one")
            )
            .0
        );
    }

    #[test]
    fn outside_a_repository_with_no_harness_one_shared_key_is_named_as_such() {
        let (key, src) = resolve_session_key(None, "CHAT_SESSION_ID", &env_of(&[]), None);
        assert_eq!(key, "shared");
        assert_eq!(src, KeySource::Shared);
    }
}
