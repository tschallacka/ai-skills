// MODE: DEV
// PACKAGE: PROD
//! The client's state directory, the per-agent session-key resolution ladder,
//! and the persisted `Session` (default server+nick, per-channel cursors)
//! (T101 split out of lib.rs).

use std::fs;
use std::path::PathBuf;

// The client's state directory: --state wins, then $AI_CHAT_HOME, then the
// central XDG home. Resolved once in main() from the raw arguments rather than
// per subcommand, so it applies to `session` and `discover` too — every
// subcommand that has a state directory gets the same one.
//
// --state was advertised in usage() and parsed nowhere, which is worse than
// not offering it: `read --state /tmp/x` silently read the default directory
// while its caller believed it was isolated. Given that a shared state
// directory is what makes two agents share a nick and a cursor (B116), a flag
// that pretends to separate them and does not is the wrong failure.
pub fn client_state_dir(args: &[String]) -> PathBuf {
    if let Some(dir) = parse_flag(args, "--state") {
        if !dir.trim().is_empty() {
            return PathBuf::from(dir);
        }
    }
    std::env::var("AI_CHAT_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| chat_default_home())
}

// The central state home everything global shares: the XDG config home's
// tsch-ai-skills directory, beside the shared bin/ and the global plans.
pub fn chat_default_home() -> PathBuf {
    match std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|v| !v.is_empty())
    {
        Some(v) => PathBuf::from(v).join("tsch-ai-skills").join("chat"),
        None => dirs_home()
            .join(".config")
            .join("tsch-ai-skills")
            .join("chat"),
    }
}

fn dirs_home() -> PathBuf {
    // USERPROFILE is Windows' HOME: a session started outside Git for Windows'
    // bash has no HOME at all.
    PathBuf::from(
        std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_else(|_| ".".into()),
    )
}

// ---- per-agent session identity -------------------------------------------

/// Where the session key came from, so `session show` can say which rung of the
/// ladder decided and an agent can tell a shared key from its own.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum KeySource {
    /// `--session ID` or `$CHAT_SESSION_ID`.
    Explicit,
    /// A session id the coding harness itself exports.
    Harness,
    /// The git worktree root: the zero-config default for agents that each
    /// work in their own checkout of one project.
    Worktree,
    /// Nothing distinguished this agent, so it shares one session.
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

/// Harness-exported identity variables, most specific first. Each was measured
/// on this machine to be identical across repeated invocations of one agent
/// (including through `env`, `timeout` and a shell-function wrapper) and to
/// differ between genuinely different agents:
///
/// - `CLAUDE_CODE_SESSION_ID` -- Claude Code, one per session and per subagent.
/// - `CODEX_SESSION_ID` -- codex; `CODEX_THREAD_ID` was measured equal to it,
///   so it adds nothing.
/// - `OPENCODE_PID` -- opencode exports no session id, only the pid of the
///   opencode process. That is instance granularity, not session granularity:
///   several sessions inside one opencode process share it, and a recycled pid
///   can adopt a dead instance's cursors. It is still the only thing opencode
///   offers, and it is a value the harness exports rather than something read
///   back out of the process tree, so it does not move per invocation.
///
/// Every variable that is set contributes to the key, rather than the first one
/// winning. Harnesses nest: a codex launched from a Claude Code agent inherits
/// that agent's `CLAUDE_CODE_SESSION_ID` unchanged and adds its own
/// `CODEX_SESSION_ID` (measured). Taking only the first match would give the
/// inner codex the outer agent's session; combining them keeps the two apart
/// whichever way round they are nested.
const HARNESS_ID_VARS: [&str; 3] = ["CLAUDE_CODE_SESSION_ID", "CODEX_SESSION_ID", "OPENCODE_PID"];

/// FNV-1a, 64-bit. Not a cryptographic hash and does not need to be: it only
/// turns an identity string into a short, stable, filename-safe key.
fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in bytes {
        h ^= *b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// Reduce a caller-chosen id to something safe to use as a filename, keeping it
/// readable so `ls sessions/` still says whose session is whose.
fn safe_key(raw: &str) -> String {
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
    // "." and ".." would name a directory rather than a session.
    if out.is_empty() || out.chars().all(|c| c == '.') {
        return String::new();
    }
    out
}

/// Resolve which session this invocation owns, as a precedence ladder. Pure:
/// the environment and the worktree root are handed in, so each rung can be
/// tested without an actual agent, an actual harness, or an actual repository.
///
/// Deliberately absent: anything read out of the process tree. `pid`, `ppid`
/// and `getsid` were all measured to change between two invocations by the same
/// agent (a runner such as `timeout` or `env`, or the harness re-execing, gives
/// a fresh pid every call), which would mint a new session per call and lose the
/// cursors the session exists to keep. Inside codex's sandbox they are worse
/// than unstable: they are pinned at 3/2/1 for every session on the machine, so
/// they are stable and identical, which would merge every codex agent into one.
/// `nick` joins the inferred rungs, and only the inferred ones.
///
/// A Claude Code subagent is not a separate process: measured, every subagent
/// of one session shares that session's pid and its CLAUDE_CODE_SESSION_ID, so
/// the harness rung alone gives them one key. A subagent joining under its own
/// nick then wrote into the parent's session file and moved the parent's
/// cursors, which is unread messages lost with nothing to see.
///
/// The nick is what actually distinguishes two agents on this bus, so it is
/// folded in alongside the harness identity rather than replacing it: identity
/// alone collides between a session and its children, and nick alone would let
/// an unrelated process on the machine claim a session by picking the name.
///
/// Explicit is deliberately left alone. `--session ID` is a caller naming a
/// session, and two callers naming the same one mean to share it.
pub fn resolve_session_key(
    explicit: Option<&str>,
    env: &dyn Fn(&str) -> Option<String>,
    worktree_root: Option<&str>,
    nick: Option<&str>,
) -> (String, KeySource) {
    let nick = nick.unwrap_or("").trim();
    // 1. What Tschallacka asked for by name always wins; no inference.
    let chosen = explicit
        .map(|s| s.to_string())
        .filter(|s| !s.is_empty())
        .or_else(|| env("CHAT_SESSION_ID").filter(|s| !s.is_empty()));
    if let Some(id) = chosen {
        let key = safe_key(&id);
        if !key.is_empty() {
            return (key, KeySource::Explicit);
        }
    }

    // The nick is a visible SUFFIX on an identity that does not include it,
    // never hashed in with it. Hashing the two together (B278) left no way to
    // find a session without already knowing its nick, so `session set --nick
    // solo` wrote one file and a later `send` with no --nick opened a
    // different, empty one -- destroying the one thing a saved session is for.
    // With the identity stable, session_key() can look for the sibling.
    let suffix = match safe_key(nick) {
        n if n.is_empty() => String::new(),
        n => format!("-{n}"),
    };

    // 2. Whatever identity the harness already knows about itself. It is shared
    //    by a session and every subagent it runs, which is why the nick has to
    //    be on the end of it.
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
            format!("h-{:016x}{suffix}", fnv1a64(material.as_bytes())),
            KeySource::Harness,
        );
    }

    // 3. The worktree root. Agents on one project in separate worktrees are the
    //    case this skill exists for, and the root is already unique per
    //    checkout on a machine, so the shared repository directory would add
    //    nothing to distinctness -- two checkouts of one repo have different
    //    roots, and sibling worktrees must NOT share a session. Two agents in
    //    ONE worktree are separated by the nick and nothing else.
    if let Some(root) = worktree_root.filter(|r| !r.is_empty()) {
        return (
            format!("w-{:016x}{suffix}", fnv1a64(root.as_bytes())),
            KeySource::Worktree,
        );
    }

    // 4. Nothing to go on -- outside a repository, with no harness and no
    //    explicit id. One shared session per nick, which is the behaviour that
    //    predates this ladder for a single agent, under a name that says so.
    (format!("shared{suffix}"), KeySource::Shared)
}

/// The session this process owns, resolved once. Reading `--session` and
/// `--nick` straight out of argv keeps every existing call site unchanged: the
/// session key is a property of the invocation, like argv itself.
///
/// A subcommand carrying no `--nick` (`session show`) resolves the no-nick key,
/// which is the key a nick-less invocation would have written. That is the
/// honest answer rather than a guess at which agent is asking.
pub fn session_key() -> &'static (String, KeySource) {
    static KEY: std::sync::OnceLock<(String, KeySource)> = std::sync::OnceLock::new();
    KEY.get_or_init(|| {
        let args: Vec<String> = std::env::args().collect();
        let explicit = parse_flag(&args, "--session");
        let nick = parse_flag(&args, "--nick");
        let (key, source) = resolve_session_key(
            explicit.as_deref(),
            &|name| std::env::var(name).ok(),
            git_worktree_root().as_deref(),
            nick.as_deref(),
        );
        if nick.is_some() {
            return (key, source);
        }
        // No --nick on this call, so the key carries no nick suffix. A saved
        // session exists to spare the caller repeating --nick, so look for the
        // one this identity owns before answering with an empty session (B278).
        match adopt_sibling_session(&key) {
            Some(sibling) => (sibling, source),
            None => (key, source),
        }
    })
}

/// The single `<key>-<nick>` session belonging to `key`, when there is exactly
/// one.
///
/// Ambiguity is left alone rather than guessed at: two nicks under one identity
/// is a parent and its subagent, and picking either would reintroduce the
/// cross-writing B271 fixed. The caller passes --nick and says which it means.
fn adopt_sibling_session(key: &str) -> Option<String> {
    // The same argv slice run() hands client_state_dir, so `--state` is
    // honoured here too; reading the default root would look in the wrong
    // place for every caller that names one.
    let args: Vec<String> = std::env::args().collect();
    sibling_session_in(&client_state_dir(args.get(2..).unwrap_or(&[])), key)
}

/// The directory half, taking the state root so it can be tested without argv.
pub(crate) fn sibling_session_in(state_dir: &std::path::Path, key: &str) -> Option<String> {
    let directory = state_dir.join("sessions");
    if directory.join(format!("{key}.json")).is_file() {
        return None;
    }
    let prefix = format!("{key}-");
    let mut found: Option<String> = None;
    for entry in fs::read_dir(&directory).ok()?.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let Some(stem) = name.strip_suffix(".json") else {
            continue;
        };
        if !stem.starts_with(&prefix) {
            continue;
        }
        if found.is_some() {
            return None;
        }
        found = Some(stem.to_string());
    }
    found
}

/// The current worktree root, or None outside a git repository (or where git is
/// not installed, which must degrade to the next rung rather than fail).
fn git_worktree_root() -> Option<String> {
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

/// Persistent session state: the default server+nick and per-channel message
/// cursors, so an agent does not repeat `--server host:port --nick me` (and a
/// since-id) on every call. Stored as `<state>/sessions/<key>.json`, one file
/// per agent, so agents sharing an `AI_CHAT_HOME` do not share a nick or a
/// cursor.
#[derive(serde::Serialize, serde::Deserialize, Default)]
pub struct Session {
    pub server: String,
    pub nick: String,
    #[serde(default)]
    pub cursors: std::collections::HashMap<String, u64>, // #chan -> last seen id
}

impl Session {
    pub fn path(state_dir: &std::path::Path) -> PathBuf {
        Session::path_for(state_dir, &session_key().0)
    }

    pub(crate) fn path_for(state_dir: &std::path::Path, key: &str) -> PathBuf {
        state_dir.join("sessions").join(format!("{}.json", key))
    }

    /// The pre-ladder location: one session for the whole state dir.
    pub(crate) fn legacy_path(state_dir: &std::path::Path) -> PathBuf {
        state_dir.join("session.json")
    }

    /// Load the session under this process's own resolved key -- see
    /// `load_with_key` for the recovery behavior, identical here.
    pub fn load(state_dir: &std::path::Path) -> Session {
        Session::load_with_key(state_dir, &session_key().0)
    }

    /// Load the session stored under an explicit `key` rather than this
    /// process's own resolved `session_key()` -- what a caller multiplexing
    /// several identities in one process (chat-mcp, T143) uses to keep each
    /// one's server/nick/cursors apart, without disturbing anything that
    /// still calls `load` unkeyed.
    ///
    /// Recovers from a missing or malformed file. A malformed file is
    /// reported on stderr (so an agent knows the cursors were reset) and an
    /// empty session is returned; the next save overwrites it.
    ///
    /// An agent that has no session file of its own yet inherits the old
    /// shared `session.json` if one is there, so an upgrade mid-conversation
    /// does not drop the nick and cursors an agent was already using. The
    /// shared file is only read, never moved or rewritten: every agent still
    /// holding state in it needs it to stay put, and each writes to its own
    /// file from then on.
    pub fn load_with_key(state_dir: &std::path::Path, key: &str) -> Session {
        let mut path = Session::path_for(state_dir, key);
        if !path.exists() {
            let legacy = Session::legacy_path(state_dir);
            if legacy.exists() {
                path = legacy;
            }
        }
        let raw = match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => return Session::default(), // no file yet
        };
        match serde_json::from_str(&raw) {
            Ok(s) => s,
            Err(_) => {
                eprintln!(
                    "chat-client-rs: session file {} is malformed; starting a fresh session",
                    path.display()
                );
                Session::default()
            }
        }
    }

    pub fn save(&self, state_dir: &std::path::Path) -> std::io::Result<()> {
        self.save_with_key(state_dir, &session_key().0)
    }

    /// Save under an explicit `key` rather than this process's own resolved
    /// `session_key()` -- see `load_with_key`.
    pub fn save_with_key(&self, state_dir: &std::path::Path, key: &str) -> std::io::Result<()> {
        let path = Session::path_for(state_dir, key);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".into());
        fs::write(path, json)
    }

    pub fn cursor(&self, chan: &str) -> u64 {
        self.cursors.get(chan).copied().unwrap_or(0)
    }

    /// The recorded cursor, separating "recorded as 0" from "not recorded"
    /// (B269). `cursor` collapses both to 0.
    ///
    /// A recorded 0 is a real position, not a missing one: `join` seeds the
    /// cursor from LASTID, so 0 means the channel was empty when this agent
    /// joined and everything from id 1 is new to it. A reader that treats 0 as
    /// "nothing recorded" skips to the channel's current end instead, and every
    /// message posted after that join is lost to `read` for good.
    ///
    /// It does not open the history gate. Joining a channel that already holds
    /// 500 messages records 500 and reads 501 onward; reading further back
    /// stays an explicit `--since`, so a long-lived channel cannot flood a
    /// context by accident.
    pub fn cursor_recorded(&self, chan: &str) -> Option<u64> {
        self.cursors.get(chan).copied()
    }
}

/// Fill missing command options from the session (if a session is active).
/// Returns (server, nick) with the session's values where the caller left them
/// empty, and whether the session was consulted.
pub fn apply_session(
    server: &str,
    nick: &str,
    state_dir: &std::path::Path,
    no_session: bool,
) -> (String, String, bool) {
    apply_session_with_key(server, nick, state_dir, no_session, &session_key().0)
}

/// `apply_session`, loading under an explicit `key` rather than this
/// process's own resolved `session_key()` -- see `Session::load_with_key`.
pub fn apply_session_with_key(
    server: &str,
    nick: &str,
    state_dir: &std::path::Path,
    no_session: bool,
    key: &str,
) -> (String, String, bool) {
    if no_session {
        return (server.to_string(), nick.to_string(), false);
    }
    let s = Session::load_with_key(state_dir, key);
    let server = if server.is_empty() {
        s.server.clone()
    } else {
        server.to_string()
    };
    let nick = if nick.is_empty() {
        s.nick.clone()
    } else {
        nick.to_string()
    };
    let used = !s.server.is_empty() || !s.nick.is_empty();
    (server, nick, used)
}

/// Record a sent/received message id as the channel cursor in the session.
pub fn save_cursor(state_dir: &std::path::Path, chan: &str, id: u64, no_session: bool) {
    save_cursor_with_key(state_dir, &session_key().0, chan, id, no_session)
}

/// `save_cursor`, keyed explicitly rather than under this process's own
/// resolved `session_key()` -- see `Session::save_with_key`.
pub fn save_cursor_with_key(
    state_dir: &std::path::Path,
    key: &str,
    chan: &str,
    id: u64,
    no_session: bool,
) {
    if no_session {
        return;
    }
    let mut s = Session::load_with_key(state_dir, key);
    if id > s.cursor(chan) {
        s.cursors.insert(chan.to_string(), id);
        let _ = s.save_with_key(state_dir, key);
    }
}

/// Remember the server+nick for later calls.
pub fn save_session(state_dir: &std::path::Path, server: &str, nick: &str) {
    save_session_with_key(state_dir, &session_key().0, server, nick)
}

/// `save_session`, keyed explicitly rather than under this process's own
/// resolved `session_key()` -- see `Session::save_with_key`.
pub fn save_session_with_key(state_dir: &std::path::Path, key: &str, server: &str, nick: &str) {
    let mut s = Session::load_with_key(state_dir, key);
    if !server.is_empty() {
        s.server = server.to_string();
    }
    if !nick.is_empty() {
        s.nick = nick.to_string();
    }
    let _ = s.save_with_key(state_dir, key);
}

pub(crate) fn parse_flag(args: &[String], name: &str) -> Option<String> {
    let mut i = 0;
    while i < args.len() {
        if args[i] == name {
            return args.get(i + 1).cloned();
        }
        i += 1;
    }
    None
}
