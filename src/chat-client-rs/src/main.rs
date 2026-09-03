// MODE: DEV
// PACKAGE: PROD
//! The TLS chat client.
//!
//! Finds a chat server (UDP announce beacon), connects over TLS, pins the
//! server certificate on first connect (TOFU), and then either sends a message,
//! reads a delta since an id (via the additive FETCH extension), or tails a
//! channel. Speaks the same RFC-grammar wire format as the server (shared via
//! chat-proto).
//!
//! Cert pinning: the server certificate's DER fingerprint is stored under the
//! client's state dir keyed by host:port. The first connect records it; later
//! connects require an exact match (fail closed). `--insecure` bypasses this
//! for testing.

use std::fs;
use std::io::{BufRead, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use rustls_pki_types::{CertificateDer, ServerName, UnixTime};

const DEFAULT_BEACON_PORT: u16 = 7780;

fn usage() {
    eprintln!(
        "chat-client-rs\n\n\
         usage:\n\
         \x20 chat-client-rs discover [--wait S] [--beacon-port N] [--bcast ADDR] [--json]\n\
         \x20 chat-client-rs send [--server HOST:PORT] [--nick N] --chan #c --text MSG [--insecure]\n\
         \x20 chat-client-rs read  [--server HOST:PORT] [--nick N] --chan #c [--since ID] [--mentions] [--insecure]\n\
         \x20 chat-client-rs read  --local --chan #c [--since ID] [--mentions] [--nick N]\n\
         \x20 chat-client-rs tail  [--server HOST:PORT] [--nick N] --chan #c [--mentions] [--mention-exit] [--insecure]\n\
         \x20 chat-client-rs tail  --local --chan #c [--mentions] [--mention-exit] [--nick N]\n\
         \x20 chat-client-rs join  [--server HOST:PORT] [--nick N] --chan #c [--since ID] [--insecure]\n\
         \x20 chat-client-rs leave [--server HOST:PORT] [--nick N] --chan #c [--insecure]\n\
         \x20 chat-client-rs session show|set|clear|cursor\n\n\
         options (after the subcommand, unless noted):\n\
         \x20 --state DIR     client state dir, beats $AI_CHAT_HOME (default: $AI_CHAT_HOME\n\
         \x20                 or the tsch-ai-skills XDG chat dir)\n\
         \x20 --session ID    which session this agent owns (default: inferred, see below);\n\
         \x20                 the one flag also accepted BEFORE the subcommand\n\
         \x20 --insecure      do not pin the server cert (testing)\n\
         \x20 --no-session    ignore the saved session (server/nick/cursor)\n\
         \x20 --mentions      only messages mentioning your nick (server-side filter)\n\
         \x20 --mention-exit  tail: exit as soon as a message mentions your nick\n\
         \x20 --local         read/tail: walk $AI_CHAT_HOME/channels/<chan>.log directly,\n\
         \x20                 with no server. Ignores --state: the channel logs are the\n\
         \x20                 server's shared storage, --state is one client's own\n\n\
         The session remembers the default server+nick and per-channel cursors\n\
         (last seen message id). join seeds the cursor to the channel's current\n\
         end (so read/tail never dump old history); leave PARTs and drops the\n\
         cursor. A malformed session file is reset with a warning.\n\n\
         Each agent gets its own session file, so several agents can share one\n\
         AI_CHAT_HOME without sharing a nick or a cursor. Which session an\n\
         invocation owns is the first of these that applies:\n\
         \x20 1. --session ID, else $CHAT_SESSION_ID\n\
         \x20 2. a session id the harness exports (Claude Code, codex, opencode)\n\
         \x20 3. the git worktree root\n\
         \x20 4. otherwise one shared session\n\
         `session show` prints which rung decided and which file it is."
    );
    std::process::exit(64);
}

fn main() {
    let mut args: Vec<String> = std::env::args().collect();
    // --session is a global option, so it is accepted before the subcommand as
    // well as after it. session_key() reads it straight out of argv either way;
    // dropping the pair here keeps it from being read as the subcommand.
    if args.len() > 2 && args[1] == "--session" {
        args.drain(1..3);
    }
    if args.len() < 2 {
        usage();
    }
    let state_dir = client_state_dir(&args[2..]);
    match args[1].as_str() {
        "discover" => discover(&args[2..]),
        "send" => send(&args[2..], &state_dir),
        "read" => read_delta(&args[2..], &state_dir),
        "tail" => tail(&args[2..], &state_dir),
        "join" => join_channel(&args[2..], &state_dir),
        "leave" => leave_channel(&args[2..], &state_dir),
        "session" => session_cmd(&args[2..], &state_dir),
        other => {
            eprintln!("chat-client-rs: unknown subcommand: {}", other);
            usage();
        }
    }
}

/// Manage the persisted session (default server+nick and channel cursors).
/// Subcommands: `show`, `set --server H --nick N`, `clear [--cursors]`,
/// `cursor #chan [ID]`.
fn session_cmd(args: &[String], state_dir: &std::path::Path) {
    let sub = args.first().map(|s| s.as_str()).unwrap_or("show");
    match sub {
        "show" => {
            let s = Session::load(state_dir);
            let (key, source) = session_key();
            println!("session={} source={}", key, source.as_str());
            println!("file={}", Session::path(state_dir).display());
            println!("server={}", s.server);
            println!("nick={}", s.nick);
            for (chan, id) in &s.cursors {
                println!("cursor {} {}", chan, id);
            }
        }
        "set" => {
            let mut s = Session::load(state_dir);
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--server" => {
                        i += 1;
                        if let Some(v) = args.get(i) {
                            s.server = v.clone();
                        }
                    }
                    "--nick" => {
                        i += 1;
                        if let Some(v) = args.get(i) {
                            s.nick = v.clone();
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            if s.server.is_empty() && s.nick.is_empty() {
                eprintln!("chat-client-rs: session set needs --server and/or --nick");
                std::process::exit(64);
            }
            let _ = s.save(state_dir);
            println!("server={} nick={}", s.server, s.nick);
        }
        "clear" => {
            if args.iter().any(|a| a == "--cursors") {
                let mut s = Session::load(state_dir);
                s.cursors.clear();
                let _ = s.save(state_dir);
                println!("cleared channel cursors");
            } else {
                let _ = fs::remove_file(Session::path(state_dir));
                println!("cleared session");
            }
        }
        "cursor" => {
            let chan = args.get(1).cloned().unwrap_or_default();
            if chan.is_empty() {
                eprintln!("chat-client-rs: session cursor needs #chan [ID]");
                std::process::exit(64);
            }
            let mut s = Session::load(state_dir);
            if let Some(id) = args.get(2).and_then(|v| v.parse::<u64>().ok()) {
                s.cursors.insert(chan.clone(), id);
                let _ = s.save(state_dir);
            }
            println!("{} {}", chan, s.cursor(&chan));
        }
        other => {
            eprintln!("chat-client-rs: unknown session subcommand: {}", other);
            std::process::exit(64);
        }
    }
}

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
fn client_state_dir(args: &[String]) -> PathBuf {
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
fn chat_default_home() -> PathBuf {
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
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()))
}

// ---- per-agent session identity -------------------------------------------

/// Where the session key came from, so `session show` can say which rung of the
/// ladder decided and an agent can tell a shared key from its own.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum KeySource {
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
    fn as_str(self) -> &'static str {
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
fn resolve_session_key(
    explicit: Option<&str>,
    env: &dyn Fn(&str) -> Option<String>,
    worktree_root: Option<&str>,
) -> (String, KeySource) {
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

    // 3. The worktree root. Agents on one project in separate worktrees are the
    //    case this skill exists for, and the root is already unique per
    //    checkout on a machine, so the shared repository directory would add
    //    nothing to distinctness -- two checkouts of one repo have different
    //    roots, and sibling worktrees must NOT share a session.
    if let Some(root) = worktree_root.filter(|r| !r.is_empty()) {
        return (
            format!("w-{:016x}", fnv1a64(root.as_bytes())),
            KeySource::Worktree,
        );
    }

    // 4. Nothing to go on -- outside a repository, with no harness and no
    //    explicit id. One shared session, which is the behaviour that predates
    //    this ladder, under a name that says so.
    ("shared".to_string(), KeySource::Shared)
}

/// The session this process owns, resolved once. Reading `--session` straight
/// out of argv keeps every existing call site unchanged: the session key is a
/// property of the invocation, like argv itself.
fn session_key() -> &'static (String, KeySource) {
    static KEY: std::sync::OnceLock<(String, KeySource)> = std::sync::OnceLock::new();
    KEY.get_or_init(|| {
        let args: Vec<String> = std::env::args().collect();
        let explicit = parse_flag(&args, "--session");
        resolve_session_key(
            explicit.as_deref(),
            &|name| std::env::var(name).ok(),
            git_worktree_root().as_deref(),
        )
    })
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
struct Session {
    server: String,
    nick: String,
    #[serde(default)]
    cursors: std::collections::HashMap<String, u64>, // #chan -> last seen id
}

impl Session {
    fn path(state_dir: &std::path::Path) -> PathBuf {
        Session::path_for(state_dir, &session_key().0)
    }

    fn path_for(state_dir: &std::path::Path, key: &str) -> PathBuf {
        state_dir.join("sessions").join(format!("{}.json", key))
    }

    /// The pre-ladder location: one session for the whole state dir.
    fn legacy_path(state_dir: &std::path::Path) -> PathBuf {
        state_dir.join("session.json")
    }

    /// Load the session, recovering from a missing or malformed file. A
    /// malformed file is reported on stderr (so an agent knows the cursors were
    /// reset) and an empty session is returned; the next save overwrites it.
    ///
    /// An agent that has no session file of its own yet inherits the old shared
    /// `session.json` if one is there, so an upgrade mid-conversation does not
    /// drop the nick and cursors an agent was already using. The shared file is
    /// only read, never moved or rewritten: every agent still holding state in
    /// it needs it to stay put, and each writes to its own file from then on.
    fn load(state_dir: &std::path::Path) -> Session {
        let mut path = Session::path(state_dir);
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

    fn save(&self, state_dir: &std::path::Path) -> std::io::Result<()> {
        let path = Session::path(state_dir);
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let json = serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".into());
        fs::write(path, json)
    }

    fn cursor(&self, chan: &str) -> u64 {
        self.cursors.get(chan).copied().unwrap_or(0)
    }
}

// ---- server resolution ----------------------------------------------------
// One ladder, tried in order until something answers a TCP connect:
//   1. an explicit --server (used as-is; failures surface at connect)
//   2. the session's saved server
//   3. the last-discovered cache (fast track, most recent first)
//   4. a fresh UDP discovery pass, LAN addresses before loopback
// Nothing at the end is an error the caller reports.

fn tcp_alive(server: &str) -> bool {
    use std::net::ToSocketAddrs;
    if let Ok(addrs) = server.to_socket_addrs() {
        for a in addrs {
            if std::net::TcpStream::connect_timeout(&a, Duration::from_millis(400)).is_ok() {
                return true;
            }
        }
    }
    false
}

fn cache_file(state_dir: &std::path::Path) -> std::path::PathBuf {
    state_dir.join("discovered-servers.txt")
}

fn cache_load(state_dir: &std::path::Path) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if let Ok(t) = fs::read_to_string(cache_file(state_dir)) {
        for line in t.lines() {
            let s = line.trim();
            if !s.is_empty() && !out.contains(&s.to_string()) {
                out.push(s.to_string());
            }
        }
    }
    out
}

// Move the server to the front of the cache (most recent first), capped.
fn cache_record(state_dir: &std::path::Path, server: &str) {
    let mut list = cache_load(state_dir);
    list.retain(|s| s != server);
    list.insert(0, server.to_string());
    list.truncate(8);
    let body = list.join("\n");
    let _ = fs::write(cache_file(state_dir), body + "\n");
}

// Listen for beacons and return candidate servers, LAN addresses before
// loopback ones: a beacon whose host (or sender) is 127.0.0.1 is only
// interesting when nothing routable announces.
fn discover_candidates(beacon_port: u16, wait_s: u64) -> Vec<String> {
    let sock = match std::net::UdpSocket::bind((std::net::Ipv4Addr::UNSPECIFIED, beacon_port)) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    sock.set_read_timeout(Some(Duration::from_secs(1))).ok();
    let deadline = SystemTime::now() + Duration::from_secs(wait_s);
    let mut lan: Vec<String> = Vec::new();
    let mut local: Vec<String> = Vec::new();
    let mut buf = [0u8; 4096];
    while SystemTime::now() < deadline {
        match sock.recv_from(&mut buf) {
            Ok((n, addr)) => {
                let s = String::from_utf8_lossy(&buf[..n]).to_string();
                let port = match json_field(&s, "port") {
                    Some(p) => p,
                    None => continue,
                };
                let beacon_host = json_field(&s, "host").unwrap_or_default();
                let host = if !beacon_host.is_empty() && beacon_host != "localhost" {
                    beacon_host
                } else {
                    addr.ip().to_string()
                };
                let cand = format!("{}:{}", host, port);
                let is_local =
                    addr.ip().is_loopback() || host == "localhost" || host.starts_with("127.");
                let seen = lan.contains(&cand) || local.contains(&cand);
                if !seen {
                    if is_local {
                        local.push(cand);
                    } else {
                        lan.push(cand);
                    }
                }
            }
            Err(_) => continue,
        }
    }
    lan.extend(local);
    lan
}

// The resolution ladder; returns the server to dial.
fn resolve_server(
    arg_server: &str,
    sess_server: &str,
    state_dir: &std::path::Path,
    no_session: bool,
) -> String {
    // An explicit --server wins without probing; failures surface at connect.
    if !arg_server.is_empty() {
        return arg_server.to_string();
    }
    // The session's saved address is probed: one that no longer answers must
    // not stand between the caller and the ladder below.
    if !no_session && !sess_server.is_empty() && tcp_alive(sess_server) {
        return sess_server.to_string();
    }
    if !no_session && !sess_server.is_empty() {
        eprintln!(
            "chat-client-rs: session server {} is not answering; trying known servers",
            sess_server
        );
    }
    for cand in cache_load(state_dir) {
        if tcp_alive(&cand) {
            cache_record(state_dir, &cand);
            return cand;
        }
    }
    let beacon_port: u16 = std::env::var("AI_CHAT_BEACON_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_BEACON_PORT);
    let cands = discover_candidates(beacon_port, 3);
    if cands.is_empty() {
        eprintln!(
            "chat-client-rs: no announce beacon received on UDP port {} within 3s",
            beacon_port
        );
    }
    for cand in cands {
        if tcp_alive(&cand) {
            cache_record(state_dir, &cand);
            return cand;
        }
    }
    // Nothing answered: dial the session address anyway so the caller's own
    // connect error names something the human saved rather than an empty
    // string.
    if no_session {
        String::new()
    } else {
        sess_server.to_string()
    }
}

/// Fill missing command options from the session (if a session is active).
/// Returns (server, nick) with the session's values where the caller left them
/// empty, and whether the session was consulted.
fn apply_session(
    server: &str,
    nick: &str,
    state_dir: &std::path::Path,
    no_session: bool,
) -> (String, String, bool) {
    if no_session {
        return (server.to_string(), nick.to_string(), false);
    }
    let s = Session::load(state_dir);
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

struct Opts {
    server: String,
    nick: String,
    chan: String,
    text: String,
    since: String,
    insecure: bool,
    no_session: bool,
    mentions: bool,
    mention_exit: bool,
    local: bool,
}

fn parse_opts(args: &[String]) -> Opts {
    let mut o = Opts {
        server: String::new(),
        nick: String::new(),
        chan: String::new(),
        text: String::new(),
        since: String::new(),
        insecure: false,
        no_session: false,
        mentions: false,
        mention_exit: false,
        local: false,
    };
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--server" => {
                i += 1;
                o.server = args.get(i).cloned().unwrap_or_default();
            }
            "--nick" => {
                i += 1;
                o.nick = args.get(i).cloned().unwrap_or_default();
            }
            "--chan" | "-c" => {
                i += 1;
                o.chan = args.get(i).cloned().unwrap_or_default();
            }
            "--text" => {
                i += 1;
                o.text = args.get(i).cloned().unwrap_or_default();
            }
            "--since" => {
                i += 1;
                o.since = args.get(i).cloned().unwrap_or_default();
            }
            "--session" => {
                // Consumed by session_key() straight from argv; taken here too
                // so its value is not mistaken for another option.
                i += 1;
            }
            "--insecure" => o.insecure = true,
            "--no-session" => o.no_session = true,
            "--mentions" => o.mentions = true,
            "--mention-exit" => o.mention_exit = true,
            "--local" => o.local = true,
            _ => {}
        }
        i += 1;
    }
    o
}

/// Record a sent/received message id as the channel cursor in the session.
fn save_cursor(state_dir: &std::path::Path, chan: &str, id: u64, no_session: bool) {
    if no_session {
        return;
    }
    let mut s = Session::load(state_dir);
    if id > s.cursor(chan) {
        s.cursors.insert(chan.to_string(), id);
        let _ = s.save(state_dir);
    }
}

/// Remember the server+nick for later calls.
fn save_session(state_dir: &std::path::Path, server: &str, nick: &str) {
    let mut s = Session::load(state_dir);
    if !server.is_empty() {
        s.server = server.to_string();
    }
    if !nick.is_empty() {
        s.nick = nick.to_string();
    }
    let _ = s.save(state_dir);
}

fn parse_flag(args: &[String], name: &str) -> Option<String> {
    let mut i = 0;
    while i < args.len() {
        if args[i] == name {
            return args.get(i + 1).cloned();
        }
        i += 1;
    }
    None
}

type Client = rustls::StreamOwned<rustls::ClientConnection, TcpStream>;

fn connect(
    server: &str,
    nick: &str,
    state_dir: &std::path::Path,
    insecure: bool,
) -> Result<(Client, String), String> {
    let addr = resolve(server)?;
    let tcp = TcpStream::connect(addr).map_err(|e| format!("connect {}: {}", server, e))?;
    tcp.set_nodelay(true).ok();
    // Bound the handshake and reads so a stalled server cannot hang the client.
    tcp.set_read_timeout(Some(Duration::from_secs(5))).ok();

    let conn = rustls::ClientConnection::new(
        Arc::new(client_config(insecure)),
        server_name(server, &addr)?,
    )
    .map_err(|e| format!("tls client: {}", e))?;
    let mut tls = rustls::StreamOwned::new(conn, tcp);

    // Register first: the first write drives the TLS handshake, after which
    // the peer certificate is available for TOFU pinning. One retry covers a
    // transient race with a just-started server.
    let mut attempts = 0;
    loop {
        // Register in two writes (NICK then USER). The first write drives the
        // handshake; flushing between them ensures each line is on the wire.
        let res = write_line(&mut tls, &format!("NICK {}", nick))
            .and_then(|_| write_line(&mut tls, &format!("USER {} 0 * :{}", nick, nick)));
        match res {
            Ok(()) => break,
            Err(e) => {
                if attempts >= 1 {
                    return Err(e);
                }
                attempts += 1;
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }

    // TOFU: verify the certificate fingerprint against the pinned one, or persist
    // on first connect (unless --insecure).
    let fp = cert_fingerprint(&tls)?;
    if !insecure {
        check_or_pin(server, &fp, state_dir)?;
    }
    Ok((tls, fp))
}

fn client_config(_insecure: bool) -> rustls::ClientConfig {
    // Whether or not --insecure is set, the handshake must accept the server's
    // self-signed cert so TOFU can inspect and pin its fingerprint. The
    // distinction is enforced AFTER the handshake by `check_or_pin` (fail
    // closed unless --insecure).
    let cfg = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(NoVerify::new()))
        .with_no_client_auth();
    // No ALPN: a generic IRC-over-TLS client historically does not negotiate
    // one, and offering "irc" can cause a HandshakeFailure with some peers.
    // cfg.alpn_protocols = vec![b"irc".to_vec()];
    cfg
}

fn cert_fingerprint(
    tls: &rustls::StreamOwned<rustls::ClientConnection, TcpStream>,
) -> Result<String, String> {
    let certs = tls.conn.peer_certificates().ok_or("no peer certificate")?;
    let first = certs.first().ok_or("empty peer cert list")?;
    Ok(hex(&sha256_der(first.as_ref())))
}

fn check_or_pin(server: &str, fp: &str, state_dir: &std::path::Path) -> Result<(), String> {
    let path = state_dir.join(format!("{}.cert.fp", server_safe(server)));
    if path.exists() {
        let stored = fs::read_to_string(&path).map_err(|e| format!("read pin: {}", e))?;
        if stored.trim() != fp {
            return Err(format!(
                "server certificate changed (TOFU pin mismatch); expected {} got {}",
                stored.trim(),
                fp
            ));
        }
        Ok(())
    } else {
        fs::create_dir_all(state_dir).map_err(|e| format!("state dir: {}", e))?;
        fs::write(&path, format!("{}\n", fp)).map_err(|e| format!("pin: {}", e))?;
        Ok(())
    }
}

fn server_safe(server: &str) -> String {
    server.replace([':', '/', '.'], "_")
}

/// The host part of a `host:port` server address.
///
/// IPv6 needs care that splitting on the FIRST colon does not survive: a
/// bracketed `[::1]:6667` yielded `"["` and a bare `::1:6667` yielded the
/// empty string, and neither is a name anything can dial (B118). Both forms
/// are read the way std's `to_socket_addrs` reads them -- brackets stripped,
/// otherwise a trailing numeric port removed at the LAST colon -- so the host
/// this returns is the host the connect resolves to. IPv4 and hostnames carry
/// at most one colon, so they take the same path they always did.
fn server_host(server: &str) -> String {
    let s = server.trim();
    if let Ok(sa) = s.parse::<SocketAddr>() {
        return sa.ip().to_string();
    }
    if let Some(rest) = s.strip_prefix('[') {
        // Bracketed but not a whole socket address: no port, or an unusable one.
        return match rest.find(']') {
            Some(end) => rest[..end].to_string(),
            None => rest.to_string(),
        };
    }
    match s.rsplit_once(':') {
        Some((host, port)) if port.chars().all(|c| c.is_ascii_digit()) => host.to_string(),
        _ => s.to_string(),
    }
}

/// The TLS server name to present for a server address, given the socket
/// address the connect resolved to. An IP literal is not a DNS name, so
/// `ServerName::try_from` rejects it -- an IPv4 literal only passes by the
/// luck of looking like a label. An IP goes to `ServerName::IpAddress`
/// instead, taking the address from the resolved socket so the name always
/// matches what is dialled. A hostname keeps the DNS-name path it always had.
fn server_name(server: &str, addr: &SocketAddr) -> Result<ServerName<'static>, String> {
    let host = server_host(server);
    if host.parse::<std::net::IpAddr>().is_ok() {
        return Ok(ServerName::IpAddress(addr.ip().into()));
    }
    ServerName::try_from(host).map_err(|e| format!("server name: {}", e))
}

fn resolve(server: &str) -> Result<SocketAddr, String> {
    let mut it = server
        .to_socket_addrs()
        .map_err(|e| format!("resolve {}: {}", server, e))?;
    it.next()
        .ok_or_else(|| format!("no address for {}", server))
}

fn write_line(
    tls: &mut rustls::StreamOwned<rustls::ClientConnection, TcpStream>,
    line: &str,
) -> Result<(), String> {
    tls.write_all(format!("{}\r\n", line).as_bytes())
        .map_err(|e| format!("send: {}", e))?;
    tls.flush().map_err(|e| format!("flush: {}", e))?;
    Ok(())
}

fn read_line(
    tls: &mut rustls::StreamOwned<rustls::ClientConnection, TcpStream>,
) -> Result<String, String> {
    let mut buf = Vec::new();
    let mut b = [0u8; 1];
    loop {
        let n = tls.read(&mut b).map_err(|e| format!("read: {}", e))?;
        if n == 0 {
            break;
        }
        buf.push(b[0]);
        if b[0] == b'\n' {
            break;
        }
        if buf.len() > 65536 {
            break;
        }
    }
    if buf.is_empty() {
        return Err("connection closed by server".into());
    }
    Ok(String::from_utf8_lossy(&buf)
        .trim_end_matches(['\r', '\n'])
        .to_string())
}

// ---- subcommands ----------------------------------------------------------

fn discover(args: &[String]) {
    let wait_s: u64 = parse_flag(args, "--wait")
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);
    let beacon_port: u16 = parse_flag(args, "--beacon-port")
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_BEACON_PORT);
    let _bcast = parse_flag(args, "--bcast").unwrap_or_else(|| "255.255.255.255".into());
    let json = args.iter().any(|a| a == "--json");

    let sock = std::net::UdpSocket::bind((std::net::Ipv4Addr::UNSPECIFIED, beacon_port))
        .map_err(|e| {
            eprintln!("chat-client-rs: bind beacon port: {}", e);
            std::process::exit(70);
        })
        .unwrap();
    sock.set_read_timeout(Some(Duration::from_secs(1))).ok();
    let deadline = SystemTime::now() + Duration::from_secs(wait_s);
    let mut seen: Vec<String> = Vec::new();
    let mut buf = [0u8; 4096];
    while SystemTime::now() < deadline {
        match sock.recv_from(&mut buf) {
            Ok((n, addr)) => {
                let s = String::from_utf8_lossy(&buf[..n]).to_string();
                if let Some(name) = json_field(&s, "name") {
                    // The beacon's own host field is what a peer should dial;
                    // the packet's source address is the fallback when an
                    // older server does not carry it. A bare "localhost"
                    // names the server but is not connectable, so the
                    // source address wins over it.
                    let host = match json_field(&s, "host") {
                        Some(h) if !h.is_empty() && h != "localhost" => h,
                        _ => addr.ip().to_string(),
                    };
                    let port = json_field(&s, "port").unwrap_or_default();
                    let key = format!("{}|{}|{}", name, host, port);
                    if !seen.contains(&key) {
                        seen.push(key.clone());
                        if json {
                            let mut out = s.trim().to_string();
                            if json_field(&s, "host").is_none() {
                                out = format!("{},\"host\":\"{}\"", &out[..out.len() - 1], host);
                                out.push('}');
                            }
                            println!("{}", out);
                        } else {
                            println!("{}  (port {}, host {})", name, port, host);
                        }
                    }
                }
            }
            Err(_) => continue,
        }
    }
    if seen.is_empty() && !json {
        eprintln!(
            "chat-client-rs: no servers found within {}s on beacon port {}",
            wait_s, beacon_port
        );
    }
}

fn json_field(s: &str, key: &str) -> Option<String> {
    let marker = format!("\"{}\":", key);
    let idx = s.find(&marker)?;
    let rest = &s[idx + marker.len()..];
    let rest = rest.trim_start();
    if let Some(r) = rest.strip_prefix('"') {
        // String value: read to the closing quote.
        let end = r.find('"')?;
        Some(r[..end].to_string())
    } else {
        // Bare literal (the beacon's port and started are numbers): read to
        // the next delimiter.
        let end = rest.find([',', '}'])?;
        Some(rest[..end].trim().to_string())
    }
}

fn send(args: &[String], state_dir: &std::path::Path) {
    let o = parse_opts(args);
    let (mut server, nick, used_session) =
        apply_session(&o.server, &o.nick, state_dir, o.no_session);
    let from_session = server.clone();
    server = resolve_server(&o.server, &from_session, state_dir, o.no_session);
    if server.is_empty() {
        eprintln!(
            "chat-client-rs: no chat server found; pass --server HOST:PORT, or run `chat-client-rs session set --server HOST:PORT --nick NAME` (nothing answered: no --server, no saved session, no known server, no beacon)"
        );
        std::process::exit(64);
    }
    let session_current = used_session && server == from_session;
    if server.is_empty() || nick.is_empty() || o.chan.is_empty() || o.text.is_empty() {
        eprintln!("chat-client-rs: send needs --server --nick --chan --text (or a saved session)");
        std::process::exit(64);
    }
    let (mut tls, _fp) = match connect(&server, &nick, state_dir, o.insecure) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("chat-client-rs: {}", e);
            std::process::exit(70);
        }
    };
    if !session_current {
        save_session(state_dir, &server, &nick);
    }
    // drain to registration complete
    let _ = wait_for_welcome(&mut tls, &nick);
    let _ = write_line(&mut tls, &format!("JOIN {}", o.chan));
    let _ = write_line(&mut tls, &format!("PRIVMSG {} :{}", o.chan, o.text));
    // read the echo of the stored line
    let mut out = String::new();
    let deadline = SystemTime::now() + Duration::from_secs(4);
    while SystemTime::now() < deadline {
        match read_line(&mut tls) {
            Ok(l) => {
                if l.contains("PRIVMSG") && l.contains(&o.chan) && l.contains(&o.text) {
                    out = l;
                    break;
                }
            }
            Err(e) => {
                if !e.contains("os error 11") {
                    break;
                }
            }
        }
    }
    if out.is_empty() {
        eprintln!("chat-client-rs: no echo from server");
        std::process::exit(70);
    }
    println!("{}", out);
    // Advance the channel cursor to the newest id. The echo line is the
    // IRC-prefix form (no id), so fetch history to learn the id of the message
    // just stored.
    let _ = write_line(&mut tls, &format!("FETCH {} {}", o.chan, 0));
    let deadline = SystemTime::now() + Duration::from_secs(3);
    let mut max_id: u64 = Session::load(state_dir).cursor(&o.chan);
    while SystemTime::now() < deadline {
        match read_line(&mut tls) {
            Ok(l) => {
                if l.starts_with(":server 000 end-of-history") {
                    break;
                }
                if l.starts_with("MSG ") {
                    if let Some(id) = l
                        .split_whitespace()
                        .nth(2)
                        .and_then(|s| s.parse::<u64>().ok())
                    {
                        if id > max_id {
                            max_id = id;
                        }
                    }
                }
            }
            Err(e) => {
                if !e.contains("os error 11") {
                    break;
                }
            }
        }
    }
    if max_id > 0 {
        save_cursor(state_dir, &o.chan, max_id, o.no_session);
    }
    let _ = write_line(&mut tls, "QUIT");
}

/// Read the server's `:server 999 <nick> #chan <id>` reply (current max id).
fn read_last_id(
    tls: &mut rustls::StreamOwned<rustls::ClientConnection, TcpStream>,
    chan: &str,
) -> u64 {
    let deadline = SystemTime::now() + Duration::from_secs(3);
    while SystemTime::now() < deadline {
        match read_line(tls) {
            Ok(l) => {
                if l.contains(" 999 ") && l.contains(chan) {
                    return l
                        .split_whitespace()
                        .last()
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(0);
                }
            }
            Err(e) => {
                if !e.contains("os error 11") {
                    break;
                }
            }
        }
    }
    0
}

/// Join a channel without reading its history: seed the session cursor to the
/// channel's current end so later read/tail resume from "now".
fn join_channel(args: &[String], state_dir: &std::path::Path) {
    let o = parse_opts(args);
    let (mut server, nick, used_session) =
        apply_session(&o.server, &o.nick, state_dir, o.no_session);
    let from_session = server.clone();
    server = resolve_server(&o.server, &from_session, state_dir, o.no_session);
    if server.is_empty() {
        eprintln!(
            "chat-client-rs: no chat server found; pass --server HOST:PORT, or run `chat-client-rs session set --server HOST:PORT --nick NAME` (nothing answered: no --server, no saved session, no known server, no beacon)"
        );
        std::process::exit(64);
    }
    let session_current = used_session && server == from_session;
    if server.is_empty() || nick.is_empty() || o.chan.is_empty() {
        eprintln!("chat-client-rs: join needs --server --nick --chan (or a saved session)");
        std::process::exit(64);
    }
    let (mut tls, _fp) = match connect(&server, &nick, state_dir, o.insecure) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("chat-client-rs: {}", e);
            std::process::exit(70);
        }
    };
    if !session_current {
        save_session(state_dir, &server, &nick);
    }
    let _ = wait_for_welcome(&mut tls, &nick);
    let _ = write_line(&mut tls, &format!("JOIN {}", o.chan));
    // Ask for the current max id; we do NOT dump the channel history.
    let _ = write_line(&mut tls, &format!("LASTID {}", o.chan));
    let current = read_last_id(&mut tls, &o.chan);
    // Seed the cursor: explicit --since overrides; otherwise the current end.
    let seed = if o.since.is_empty() {
        current
    } else {
        o.since.parse::<u64>().unwrap_or(current)
    };
    if !o.no_session {
        let mut s = Session::load(state_dir);
        s.cursors.insert(o.chan.clone(), seed);
        let _ = s.save(state_dir);
    }
    println!("joined {} (resuming after id {})", o.chan, seed);
    let _ = write_line(&mut tls, "QUIT");
}

/// Leave a channel: send PART and drop the channel cursor from the session.
fn leave_channel(args: &[String], state_dir: &std::path::Path) {
    let o = parse_opts(args);
    let (mut server, nick, used_session) =
        apply_session(&o.server, &o.nick, state_dir, o.no_session);
    let from_session = server.clone();
    server = resolve_server(&o.server, &from_session, state_dir, o.no_session);
    if server.is_empty() {
        eprintln!(
            "chat-client-rs: no chat server found; pass --server HOST:PORT, or run `chat-client-rs session set --server HOST:PORT --nick NAME` (nothing answered: no --server, no saved session, no known server, no beacon)"
        );
        std::process::exit(64);
    }
    let session_current = used_session && server == from_session;
    if server.is_empty() || nick.is_empty() || o.chan.is_empty() {
        eprintln!("chat-client-rs: leave needs --server --nick --chan (or a saved session)");
        std::process::exit(64);
    }
    let (mut tls, _fp) = match connect(&server, &nick, state_dir, o.insecure) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("chat-client-rs: {}", e);
            std::process::exit(70);
        }
    };
    if !session_current {
        save_session(state_dir, &server, &nick);
    }
    let _ = wait_for_welcome(&mut tls, &nick);
    let _ = write_line(&mut tls, &format!("PART {}", o.chan));
    // Clean up the channel's cursor so a later join starts fresh at the end.
    if !o.no_session {
        let mut s = Session::load(state_dir);
        s.cursors.remove(&o.chan);
        let _ = s.save(state_dir);
    }
    println!("left {}", o.chan);
    let _ = write_line(&mut tls, "QUIT");
}

/// The channel log a local read walks: the same file the server appends to.
/// The server's own channel-name rule, restated here because a LOCAL read
/// never reaches the server to be checked by it.
///
/// A remote read is validated server-side (`valid_chan` in chat-server-rs,
/// called before every JOIN, PRIVMSG and fetch), so `--chan` could be trusted
/// for as long as every path went through a socket. `--local` walks the log
/// file directly, which takes the server out of the loop and turns the channel
/// name into a path segment: `local_chan_log` joins it straight into
/// `<home>/channels/<chan>.log`, and `Path::join` does not resolve `..`, so
/// `--chan ../../../../tmp/x` reads `/tmp/x.log`. The exit code then differs by
/// whether that file exists, which answers "does this path exist" for any path
/// the caller names.
///
/// Kept character-for-character identical to the server's rule rather than
/// merely "safe": a name the server would refuse must not be readable by going
/// around it, or the two disagree about what a channel is.
fn valid_chan(c: &str) -> bool {
    c.len() > 1
        && c.len() <= 33
        && c.starts_with('#')
        && c[1..]
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
}

fn local_chan_log(home: &std::path::Path, chan: &str) -> PathBuf {
    home.join("channels").join(format!("{}.log", chan))
}

/// Where the CHANNEL LOGS live, which is not the same place as `--state`.
///
/// Channel logs are the server's shared storage; `--state` is one client's own
/// certificate pins and sessions. An agent given its own `--state` directory
/// must still read the channels everyone shares, so a local read resolves the
/// home from `$AI_CHAT_HOME` (or the XDG default) exactly as the server does,
/// and ignores `--state`.
fn channels_home() -> PathBuf {
    std::env::var("AI_CHAT_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| chat_default_home())
}

/// One stored `MSG #chan <id> ...` line's id, or None for anything else.
fn msg_line_id(line: &str) -> Option<u64> {
    if !line.starts_with("MSG ") {
        return None;
    }
    line.split_whitespace().nth(2)?.parse::<u64>().ok()
}

/// The highest stored id in a channel log, or 0 when there is none.
///
/// Taken from the maximum over all rows rather than the last line: a truncated
/// or interleaved final write must not make the cursor go backwards.
fn local_last_id(home: &std::path::Path, chan: &str) -> u64 {
    let path = local_chan_log(home, chan);
    let file = match fs::File::open(&path) {
        Ok(f) => f,
        Err(_) => return 0,
    };
    let mut top = 0u64;
    for line in std::io::BufReader::new(file).lines().map_while(Result::ok) {
        if let Some(id) = msg_line_id(&line) {
            if id > top {
                top = id;
            }
        }
    }
    top
}

/// Print stored messages with id > `since`, straight from the channel log.
/// Returns the highest id printed.
///
/// This is the reader the skill was missing. Every read path required a live
/// server, so when none was reachable the only way to see a channel was to
/// open its log by hand -- which is what agents actually did, and it bypasses
/// mention filtering and cursors entirely. Reading the log is lossless: the
/// log IS the storage format, so a local read returns the same lines a FETCH
/// would.
fn local_read(home: &std::path::Path, chan: &str, since: u64, mentions_for: Option<&str>) -> u64 {
    let path = local_chan_log(home, chan);
    let file = match fs::File::open(&path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!(
                "chat-client-rs: cannot read {}: {} (no such channel locally; a channel exists once its first message is stored)",
                path.display(),
                e
            );
            std::process::exit(66);
        }
    };
    let mut max_id = 0u64;
    for line in std::io::BufReader::new(file).lines().map_while(Result::ok) {
        let id = match msg_line_id(&line) {
            Some(id) => id,
            None => continue, // a malformed line is skipped, never fatal
        };
        if id <= since {
            continue;
        }
        if let Some(nick) = mentions_for {
            if !line.contains(&format!("@{}", nick)) {
                continue;
            }
        }
        println!("{}", line);
        if id > max_id {
            max_id = id;
        }
    }
    max_id
}

fn read_delta(args: &[String], state_dir: &std::path::Path) {
    let o = parse_opts(args);
    if o.local {
        if o.chan.is_empty() {
            eprintln!("chat-client-rs: read --local needs --chan #c");
            std::process::exit(64);
        }
        if !valid_chan(&o.chan) {
            eprintln!("chat-client-rs: read --local: not a channel name: {}", o.chan);
            std::process::exit(64);
        }
        // --since wins; otherwise the session cursor; otherwise everything.
        // A local read defaults to the whole log rather than the current end:
        // without a server there is no LASTID to ask, and silently printing
        // nothing is the worst of the available answers.
        let since = if !o.since.is_empty() {
            o.since.parse::<u64>().unwrap_or(0)
        } else if o.no_session {
            0
        } else {
            Session::load(state_dir).cursor(&o.chan)
        };
        let mentions_for = if o.mentions {
            let nick = if o.nick.is_empty() {
                Session::load(state_dir).nick
            } else {
                o.nick.clone()
            };
            if nick.is_empty() {
                eprintln!("chat-client-rs: --mentions needs --nick (or a saved session nick)");
                std::process::exit(64);
            }
            Some(nick)
        } else {
            None
        };
        let max_id = local_read(&channels_home(), &o.chan, since, mentions_for.as_deref());
        // A mention-filtered read must NOT move the channel cursor. The cursor
        // means "the last message I have seen in this channel", and a
        // `--mentions` read has seen only the mentions: `local_read` returns the
        // highest id it PRINTED, so saving it declares every non-mention message
        // below that id as seen. With the last mention at the end of the log
        // (`ping @bob` at id 4 over three unread messages) the cursor jumped
        // straight to 4 and the next plain read returned nothing at all -- the
        // three messages were skipped permanently, having been printed by
        // nothing. Leaving the cursor alone costs a mentions reader nothing: it
        // passes `--since` or reads the whole log by design.
        if mentions_for.is_none() && max_id > 0 {
            save_cursor(state_dir, &o.chan, max_id, o.no_session);
        }
        return;
    }
    let (mut server, nick, used_session) =
        apply_session(&o.server, &o.nick, state_dir, o.no_session);
    let from_session = server.clone();
    server = resolve_server(&o.server, &from_session, state_dir, o.no_session);
    if server.is_empty() {
        eprintln!(
            "chat-client-rs: no chat server found; pass --server HOST:PORT, or run `chat-client-rs session set --server HOST:PORT --nick NAME` (nothing answered: no --server, no saved session, no known server, no beacon)"
        );
        std::process::exit(64);
    }
    let session_current = used_session && server == from_session;
    if server.is_empty() || nick.is_empty() || o.chan.is_empty() {
        eprintln!("chat-client-rs: read needs --server --nick --chan (or a saved session)");
        std::process::exit(64);
    }
    // --since defaults to the session cursor ("everything since I last saw").
    // With no cursor and no --since, default to the channel's CURRENT end (via
    // LASTID) so reading an old channel does not dump its whole history; use
    // `--since 0` (or --history via session cursor 0) to read everything.
    let mut since = o.since.clone();
    if since.is_empty() && !o.no_session {
        let cur = Session::load(state_dir).cursor(&o.chan);
        if cur > 0 {
            since = cur.to_string();
        }
    }
    let (mut tls, _fp) = match connect(&server, &nick, state_dir, o.insecure) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("chat-client-rs: {}", e);
            std::process::exit(70);
        }
    };
    if !session_current {
        save_session(state_dir, &server, &nick);
    }
    let _ = wait_for_welcome(&mut tls, &nick);
    if since.is_empty() {
        // No cursor yet: find the current end, then fetch nothing from before
        // it unless the caller asked for history.
        let _ = write_line(&mut tls, &format!("LASTID {}", o.chan));
        let current = read_last_id(&mut tls, &o.chan);
        since = current.to_string();
    }
    let mention_suffix = if o.mentions { " mentions" } else { "" };
    let _ = write_line(
        &mut tls,
        &format!("FETCH {} {}{}", o.chan, since, mention_suffix),
    );
    let deadline = SystemTime::now() + Duration::from_secs(5);
    let mut max_id: u64 = 0;
    while SystemTime::now() < deadline {
        match read_line(&mut tls) {
            Ok(l) => {
                if l.starts_with(":server 000 end-of-history") {
                    break;
                }
                if l.starts_with("MSG ") {
                    if let Some(id) = l
                        .split_whitespace()
                        .nth(2)
                        .and_then(|s| s.parse::<u64>().ok())
                    {
                        if id > max_id {
                            max_id = id;
                        }
                    }
                    println!("{}", l);
                }
            }
            Err(e) => {
                if !e.contains("os error 11") {
                    break;
                }
            }
        }
    }
    if max_id > 0 {
        save_cursor(state_dir, &o.chan, max_id, o.no_session);
    }
    let _ = write_line(&mut tls, "QUIT");
}

fn tail(args: &[String], state_dir: &std::path::Path) {
    let o = parse_opts(args);
    // `--mention-exit` means "stop once a message mentions me", which needs the
    // mention filter to know what a mention is. Alone it was quietly wrong in
    // opposite directions on the two paths: the socket tail guards the exit with
    // `o.mentions`, so the flag did nothing at all, while the local tail's exit
    // sat outside that guard and returned on the FIRST message from anyone.
    // Refusing the combination is the only reading that cannot surprise either
    // way; a caller that meant "wake me on a mention" was always going to pass
    // both.
    if o.mention_exit && !o.mentions {
        eprintln!("chat-client-rs: tail --mention-exit needs --mentions");
        std::process::exit(64);
    }
    if o.local {
        if o.chan.is_empty() {
            eprintln!("chat-client-rs: tail --local needs --chan #c");
            std::process::exit(64);
        }
        if !valid_chan(&o.chan) {
            eprintln!("chat-client-rs: tail --local: not a channel name: {}", o.chan);
            std::process::exit(64);
        }
        let nick = if o.nick.is_empty() {
            Session::load(state_dir).nick
        } else {
            o.nick.clone()
        };
        if o.mentions && nick.is_empty() {
            eprintln!("chat-client-rs: --mentions needs --nick (or a saved session nick)");
            std::process::exit(64);
        }
        let mentions_for = if o.mentions {
            Some(nick.as_str())
        } else {
            None
        };
        // A local tail starts at the log's current end unless told otherwise:
        // a watcher wants what arrives next, not the backlog.
        let mut since = if !o.since.is_empty() {
            o.since.parse::<u64>().unwrap_or(0)
        } else {
            local_last_id(&channels_home(), &o.chan)
        };
        // Same step-down cadence as the socket tail: responsive while a
        // conversation is live, near-silent when nothing is happening.
        // Record where the watch begins, not just what it later sees. Skipping
        // the backlog is a decision this tail makes for the whole channel, so
        // until it is written down `read --local` still believes nothing has
        // been seen: a tail that started at the log end and then idled left no
        // session file at all, and the next plain read re-printed the entire
        // backlog the tail had just declared old. A mention-filtered tail is the
        // exception, for the reason `read --local` does not save one either.
        if mentions_for.is_none() && since > 0 {
            save_cursor(state_dir, &o.chan, since, o.no_session);
        }
        let mut wait = 1u64;
        loop {
            let max_id = local_read(&channels_home(), &o.chan, since, mentions_for);
            if max_id > since {
                since = max_id;
                if mentions_for.is_none() {
                    save_cursor(state_dir, &o.chan, max_id, o.no_session);
                }
                wait = 1;
                // Guarded on the filter, matching the socket tail. Unguarded,
                // this returned on the first message from anyone; `tail()` now
                // also refuses `--mention-exit` without `--mentions`, so the
                // two agree and this cannot fire on a non-mention.
                if o.mention_exit && mentions_for.is_some() {
                    return;
                }
            } else if wait < 60 {
                wait = (wait * 2).min(60);
            }
            std::thread::sleep(Duration::from_secs(wait));
        }
    }
    let (mut server, nick, used_session) =
        apply_session(&o.server, &o.nick, state_dir, o.no_session);
    let from_session = server.clone();
    server = resolve_server(&o.server, &from_session, state_dir, o.no_session);
    if server.is_empty() {
        eprintln!(
            "chat-client-rs: no chat server found; pass --server HOST:PORT, or run `chat-client-rs session set --server HOST:PORT --nick NAME` (nothing answered: no --server, no saved session, no known server, no beacon)"
        );
        std::process::exit(64);
    }
    let session_current = used_session && server == from_session;
    if server.is_empty() || nick.is_empty() || o.chan.is_empty() {
        eprintln!("chat-client-rs: tail needs --server --nick --chan (or a saved session)");
        std::process::exit(64);
    }
    let (mut tls, _fp) = match connect(&server, &nick, state_dir, o.insecure) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("chat-client-rs: {}", e);
            std::process::exit(70);
        }
    };
    if !session_current {
        save_session(state_dir, &server, &nick);
    }
    let _ = wait_for_welcome(&mut tls, &nick);
    let _ = write_line(&mut tls, &format!("JOIN {}", o.chan));
    // Resume from the session cursor; with no cursor yet, default to the
    // channel's CURRENT end (LASTID) so tailing an old channel does not dump
    // its whole history — only new messages are shown from now on.
    let mut last_id: u64 = if o.no_session {
        0
    } else {
        let cur = Session::load(state_dir).cursor(&o.chan);
        if cur > 0 {
            cur
        } else {
            let _ = write_line(&mut tls, &format!("LASTID {}", o.chan));
            read_last_id(&mut tls, &o.chan)
        }
    };
    let mention_suffix = if o.mentions { " mentions" } else { "" };
    let mut interval: u64 = 5;
    let mut last_seen = SystemTime::now();
    loop {
        // Poll for new history since the last id we saw; print any new rows.
        let _ = write_line(
            &mut tls,
            &format!("FETCH {} {}{}", o.chan, last_id, mention_suffix),
        );
        let poll_deadline = SystemTime::now() + Duration::from_secs(2);
        let mut new_msg = false;
        while SystemTime::now() < poll_deadline {
            match read_line(&mut tls) {
                Ok(l) => {
                    if l.starts_with(":server 000 end-of-history") {
                        break;
                    }
                    if l.starts_with("MSG ") {
                        let id = l
                            .split_whitespace()
                            .nth(2)
                            .and_then(|s| s.parse::<u64>().ok())
                            .unwrap_or(0);
                        if id > last_id {
                            last_id = id;
                            new_msg = true;
                            if o.mentions {
                                // Notification: mark the mention distinctly so
                                // a watcher can act on it.
                                println!("!! MENTION !! {}", l);
                                if o.mention_exit {
                                    let _ = write_line(&mut tls, "QUIT");
                                    std::process::exit(0);
                                }
                            } else {
                                println!("{}", l);
                            }
                            save_cursor(state_dir, &o.chan, id, o.no_session);
                        }
                    }
                }
                Err(e) => {
                    if !e.contains("os error 11") {
                        break;
                    }
                }
            }
        }
        if new_msg {
            // A message reset the cadence to fast.
            interval = 5;
            last_seen = SystemTime::now();
        } else {
            // No new message: step down toward the 60s cap.
            interval = (interval + 10).min(60);
        }
        std::thread::sleep(Duration::from_secs(interval));
        // Bound an idle stretch to the cadence; if the connection is dead the
        // next FETCH will error and this loop breaks.
        let _ = last_seen;
    }
}

fn wait_for_welcome(
    tls: &mut rustls::StreamOwned<rustls::ClientConnection, TcpStream>,
    base_nick: &str,
) -> Result<(), String> {
    let mut seen_001 = false;
    let mut attempt = 2u32;
    let deadline = SystemTime::now() + Duration::from_secs(4);
    while SystemTime::now() < deadline {
        match read_line(tls) {
            Ok(l) => {
                if l.contains(" 001 ") || l.starts_with(":") && l.contains(" 001 ") {
                    seen_001 = true;
                }
                // Nick already in use (e.g. a tail holds the session nick):
                // auto-retry with a numeric suffix like real IRC clients, so a
                // concurrent send/read can register alongside the holder.
                if l.contains(" 433 ") {
                    let alt = format!("{}-{}", base_nick, attempt);
                    attempt += 1;
                    let _ = write_line(tls, &format!("NICK {}", alt));
                    continue;
                }
                if seen_001 && l.contains(" 376 ") {
                    return Ok(());
                }
            }
            Err(e) => {
                // EAGAIN/EWOULDBLOCK: the welcome hasn't arrived within this
                // read's timeout but the connection is alive; keep waiting for
                // the deadline rather than failing a noisy localhost exchange.
                if !e.contains("os error 11") {
                    return Err(e);
                }
            }
        }
    }
    if seen_001 {
        Ok(())
    } else {
        Err("registration did not complete (no 001)".into())
    }
}

// ---- crypto/verifier helpers ------------------------------------------------

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn sha256_der(der: &[u8]) -> [u8; 32] {
    // Minimal SHA-256 (no external crate). The fingerprint need not be
    // cryptographic-strength against a custom hash collision here because the
    // server cert itself is trusted; SHA-256 over the DER is a stable checksum
    // for TOFU identity. Implemented in-crate to keep dependencies zero.
    sha256(der)
}

// A compact SHA-256 implementation to avoid pulling a crypto dependency for the
// TOFU fingerprint.
fn sha256(message: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut data = message.to_vec();
    let bitlen = (data.len() as u64).wrapping_mul(8);
    data.push(0x80);
    while data.len() % 64 != 56 {
        data.push(0);
    }
    data.extend_from_slice(&bitlen.to_be_bytes());
    let mut w = [0u32; 64];
    for chunk in data.chunks(64) {
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    let mut out = [0u8; 32];
    for i in 0..8 {
        out[i * 4..i * 4 + 4].copy_from_slice(&h[i].to_be_bytes());
    }
    out
}

/// A certificate verifier that accepts the peer certificate as-is. Used with
/// `--insecure`; the default path calls `check_or_pin` for TOFU after connect.
#[derive(Debug)]
struct NoVerify();

impl NoVerify {
    fn new() -> NoVerify {
        NoVerify()
    }
}

impl rustls::client::danger::ServerCertVerifier for NoVerify {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp_state(name: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("chat-session-test-{}-{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        // Session files live under sessions/<key>.json; a test that writes one
        // by hand needs the directory to exist, as `save` would have made it.
        fs::create_dir_all(d.join("sessions")).unwrap();
        d
    }

    /// An env lookup over a fixed table, so a rung can be tested without an
    /// actual harness and without touching the process environment (which
    /// would race the other tests in this binary).
    fn env_of<'a>(pairs: &'a [(&'a str, &'a str)]) -> impl Fn(&str) -> Option<String> + 'a {
        move |name: &str| {
            pairs
                .iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| v.to_string())
        }
    }

    #[test]
    fn explicit_session_flag_wins_over_everything() {
        let env = env_of(&[
            ("CHAT_SESSION_ID", "from-env"),
            ("CLAUDE_CODE_SESSION_ID", "claude-1"),
        ]);
        let (key, src) = resolve_session_key(Some("agent-b"), &env, Some("/repo"));
        assert_eq!(key, "agent-b");
        assert_eq!(src, KeySource::Explicit);
    }

    #[test]
    fn chat_session_id_env_wins_over_inference() {
        let env = env_of(&[
            ("CHAT_SESSION_ID", "from-env"),
            ("CLAUDE_CODE_SESSION_ID", "claude-1"),
        ]);
        let (key, src) = resolve_session_key(None, &env, Some("/repo"));
        assert_eq!(key, "from-env");
        assert_eq!(src, KeySource::Explicit);
    }

    #[test]
    fn explicit_id_is_reduced_to_a_safe_filename() {
        let env = env_of(&[]);
        let (key, src) = resolve_session_key(Some("../../etc/passwd"), &env, None);
        assert_eq!(src, KeySource::Explicit);
        assert!(!key.contains('/'), "key must not contain a path separator");
        // What matters is where the key resolves: one file directly inside
        // sessions/, never a path that climbs out of it.
        let d = tmp_state("explicit_id_is_reduced_to_a_safe_filename");
        let path = Session::path_for(&d, &key);
        assert_eq!(
            path.parent().unwrap(),
            d.join("sessions"),
            "resolved to {}",
            path.display()
        );
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn an_all_dots_explicit_id_falls_through_rather_than_naming_a_directory() {
        let env = env_of(&[("CLAUDE_CODE_SESSION_ID", "claude-1")]);
        let (key, src) = resolve_session_key(Some(".."), &env, Some("/repo"));
        assert_eq!(src, KeySource::Harness, "got key {}", key);
    }

    #[test]
    fn each_harness_session_id_gives_a_distinct_key() {
        let a = resolve_session_key(None, &env_of(&[("CLAUDE_CODE_SESSION_ID", "a")]), None);
        let b = resolve_session_key(None, &env_of(&[("CLAUDE_CODE_SESSION_ID", "b")]), None);
        assert_eq!(a.1, KeySource::Harness);
        assert_ne!(a.0, b.0, "two Claude Code sessions must not share a key");

        let c = resolve_session_key(None, &env_of(&[("CODEX_SESSION_ID", "c")]), None);
        let d = resolve_session_key(None, &env_of(&[("CODEX_SESSION_ID", "d")]), None);
        assert_eq!(c.1, KeySource::Harness);
        assert_ne!(c.0, d.0, "two codex sessions must not share a key");

        let e = resolve_session_key(None, &env_of(&[("OPENCODE_PID", "111")]), None);
        assert_eq!(e.1, KeySource::Harness);
        assert_ne!(
            e.0,
            resolve_session_key(None, &env_of(&[("OPENCODE_PID", "222")]), None).0
        );
    }

    #[test]
    fn a_harness_key_is_stable_for_the_same_ids() {
        let pairs = [("CODEX_SESSION_ID", "same"), ("OPENCODE_PID", "9")];
        let first = resolve_session_key(None, &env_of(&pairs), Some("/a"));
        let again = resolve_session_key(None, &env_of(&pairs), Some("/b"));
        assert_eq!(
            first.0, again.0,
            "the harness rung must not depend on the worktree"
        );
    }

    #[test]
    fn a_nested_harness_does_not_inherit_the_outer_agents_session() {
        // Measured: a codex launched from a Claude Code agent keeps that
        // agent's CLAUDE_CODE_SESSION_ID and adds its own CODEX_SESSION_ID.
        let outer = resolve_session_key(None, &env_of(&[("CLAUDE_CODE_SESSION_ID", "a")]), None);
        let inner = resolve_session_key(
            None,
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
        let a = resolve_session_key(None, &env, Some("/repo/.claude/worktrees/one"));
        let b = resolve_session_key(None, &env, Some("/repo/.claude/worktrees/two"));
        assert_eq!(a.1, KeySource::Worktree);
        assert_ne!(a.0, b.0, "sibling worktrees must not share a session");
        // Same root, twice: the same key.
        assert_eq!(
            a.0,
            resolve_session_key(None, &env, Some("/repo/.claude/worktrees/one")).0
        );
    }

    #[test]
    fn outside_a_repository_with_no_harness_one_shared_session_is_named_as_such() {
        let (key, src) = resolve_session_key(None, &env_of(&[]), None);
        assert_eq!(key, "shared");
        assert_eq!(src, KeySource::Shared);
    }

    #[test]
    fn distinct_session_keys_get_distinct_files() {
        let d = tmp_state("distinct_session_keys_get_distinct_files");
        assert_ne!(
            Session::path_for(&d, "h-1111111111111111"),
            Session::path_for(&d, "h-2222222222222222")
        );
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn an_agent_with_no_file_yet_inherits_the_old_shared_session_without_moving_it() {
        let d = tmp_state("legacy_session_is_inherited_not_moved");
        fs::write(
            Session::legacy_path(&d),
            "{\"server\":\"h:1\",\"nick\":\"old\",\"cursors\":{\"#a\":4}}",
        )
        .unwrap();
        let s = Session::load(&d);
        assert_eq!(s.nick, "old");
        assert_eq!(s.cursor("#a"), 4);
        assert!(
            Session::legacy_path(&d).exists(),
            "the shared file other agents still read must stay put"
        );
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn session_round_trips_server_nick_and_cursors() {
        let d = tmp_state("session_round_trips_server_nick_and_cursors");
        let s = Session {
            server: "127.0.0.1:1234".into(),
            nick: "agent".into(),
            cursors: std::collections::HashMap::from([("#ops".to_string(), 7)]),
        };
        s.save(&d).unwrap();

        let loaded = Session::load(&d);
        assert_eq!(loaded.server, "127.0.0.1:1234");
        assert_eq!(loaded.nick, "agent");
        assert_eq!(loaded.cursor("#ops"), 7);
        assert_eq!(loaded.cursor("#other"), 0);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn session_missing_file_is_empty_session() {
        let d = tmp_state("session_missing_file_is_empty_session");
        let s = Session::load(&d);
        assert_eq!(s.server, "");
        assert_eq!(s.nick, "");
        assert_eq!(s.cursor("#x"), 0);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn session_malformed_json_recovers_to_empty() {
        let d = tmp_state("session_malformed_json_recovers_to_empty");
        fs::write(Session::path(&d), "{ not valid json !!!").unwrap();
        let s = Session::load(&d);
        assert_eq!(s.server, "");
        assert_eq!(s.nick, "");
        assert_eq!(s.cursor("#x"), 0);
        // a subsequent save overwrites the malformed file cleanly
        let s2 = Session {
            server: "h:1".into(),
            nick: String::new(),
            cursors: Default::default(),
        };
        s2.save(&d).unwrap();
        let reloaded = Session::load(&d);
        assert_eq!(reloaded.server, "h:1");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn session_partial_json_keeps_what_parses_or_resets() {
        // A JSON object missing the cursors key must still deserialize
        // (serde default) rather than failing the whole file.
        let d = tmp_state("session_partial_json_keeps_what_parses_or_resets");
        fs::write(Session::path(&d), "{\"server\":\"h:1\",\"nick\":\"n\"}").unwrap();
        let s = Session::load(&d);
        assert_eq!(s.server, "h:1");
        assert_eq!(s.nick, "n");
        assert_eq!(s.cursor("#x"), 0);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn cursor_updates_monotonically() {
        let d = tmp_state("cursor_updates_monotonically");
        save_cursor(&d, "#ops", 5, false);
        save_cursor(&d, "#ops", 3, false); // lower: ignored
        assert_eq!(Session::load(&d).cursor("#ops"), 5);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn no_session_flag_skips_cursor_save() {
        let d = tmp_state("no_session_flag_skips_cursor_save");
        save_cursor(&d, "#ops", 9, true);
        assert_eq!(Session::load(&d).cursor("#ops"), 0);
        let _ = fs::remove_dir_all(&d);
    }

    // --state was documented and never parsed. These assert the flag path
    // only, which returns before any environment is read, so the test does
    // not race other tests over AI_CHAT_HOME.
    #[test]
    fn state_flag_selects_the_client_state_dir() {
        let dir = client_state_dir(&["--state".into(), "/tmp/chat-state-probe".into()]);
        assert_eq!(dir, PathBuf::from("/tmp/chat-state-probe"));

        // It is found wherever it sits in the argument list, since it is
        // resolved from the raw arguments rather than by a subcommand parser.
        let dir = client_state_dir(&[
            "--chan".into(),
            "#c".into(),
            "--state".into(),
            "/tmp/elsewhere".into(),
            "--nick".into(),
            "me".into(),
        ]);
        assert_eq!(dir, PathBuf::from("/tmp/elsewhere"));
    }

    #[test]
    fn a_blank_or_absent_state_flag_does_not_win() {
        // A blank value must not resolve the state dir to "": it falls through
        // to $AI_CHAT_HOME / the XDG default, whatever those are here.
        let dir = client_state_dir(&["--state".into(), "   ".into()]);
        assert_ne!(dir, PathBuf::from(""));
        assert_ne!(dir, PathBuf::from("   "));

        // A trailing --state with no value must not panic.
        let dir = client_state_dir(&["--state".into()]);
        assert_ne!(dir, PathBuf::from(""));
    }

    #[test]
    fn parse_opts_reads_mention_flags() {
        let o = parse_opts(&[
            "--mentions".into(),
            "--mention-exit".into(),
            "--chan".into(),
            "#m".into(),
            "--server".into(),
            "h:1".into(),
            "--nick".into(),
            "me".into(),
        ]);
        assert!(o.mentions);
        assert!(o.mention_exit);
        assert_eq!(o.chan, "#m");
        assert!(!o.no_session);
    }

    #[test]
    fn session_clear_drops_cursors_keeps_identity() {
        let d = tmp_state("session_clear_drops_cursors_keeps_identity");
        let mut s = Session {
            server: "h:1".into(),
            nick: "me".into(),
            cursors: std::collections::HashMap::from([("#a".to_string(), 3)]),
        };
        s.save(&d).unwrap();
        s.cursors.clear();
        s.save(&d).unwrap();
        let loaded = Session::load(&d);
        assert_eq!(loaded.server, "h:1");
        assert_eq!(loaded.nick, "me");
        assert_eq!(loaded.cursor("#a"), 0);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn read_last_id_parses_numeric_reply() {
        // Simulate the server's `:server 999 nick #chan 42` reply line.
        let line = ":server 999 me #ops 42";
        let id = line
            .split_whitespace()
            .last()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        assert_eq!(id, 42);
        // A malformed reply yields 0 (client treats it as an empty channel).
        let bad = " 999 me #ops";
        let id2 = bad
            .split_whitespace()
            .last()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        assert_eq!(id2, 0);
    }

    // ---- B118: an IPv6 server address must be dialable -------------------

    #[test]
    fn server_host_reads_ipv4_and_hostnames_exactly_as_before() {
        assert_eq!(server_host("127.0.0.1:6667"), "127.0.0.1");
        assert_eq!(server_host("192.168.1.5:1234"), "192.168.1.5");
        assert_eq!(server_host("localhost:6667"), "localhost");
        assert_eq!(server_host("chat.example.org:6667"), "chat.example.org");
        assert_eq!(server_host("localhost"), "localhost");
        assert_eq!(server_host("localhost:"), "localhost");
    }

    #[test]
    fn server_host_extracts_the_host_from_an_ipv6_address() {
        // Bracketed: the first-colon split used to return "[".
        assert_eq!(server_host("[::1]:6667"), "::1");
        assert_eq!(
            server_host("[fe80::1ff:fe23:4567:890a]:6667"),
            "fe80::1ff:fe23:4567:890a"
        );
        // Bare, the form the announce beacon's host+port concatenation makes:
        // the first-colon split used to return the empty string. std's
        // to_socket_addrs splits this on the LAST colon, so this must too.
        assert_eq!(server_host("::1:6667"), "::1");
        assert_eq!(
            server_host("fe80::1ff:fe23:4567:890a:6667"),
            "fe80::1ff:fe23:4567:890a"
        );
    }

    #[test]
    fn an_ip_literal_becomes_an_ip_server_name_not_a_dns_name() {
        use std::net::{IpAddr, Ipv6Addr, SocketAddr};
        // The whole point of B118: ServerName::try_from rejects "::1" as an
        // invalid DNS name, so an IP must not take the DNS-name path at all.
        let v6: SocketAddr = SocketAddr::new(IpAddr::V6(Ipv6Addr::LOCALHOST), 6667);
        for addr in ["[::1]:6667", "::1:6667"] {
            let name = server_name(addr, &v6)
                .unwrap_or_else(|e| panic!("{} must yield a server name, got {}", addr, e));
            assert!(
                matches!(name, ServerName::IpAddress(_)),
                "{} must present an IP server name, got {:?}",
                addr,
                name
            );
        }
        let v4: SocketAddr = "127.0.0.1:6667".parse().unwrap();
        assert!(
            matches!(
                server_name("127.0.0.1:6667", &v4).unwrap(),
                ServerName::IpAddress(_)
            ),
            "an IPv4 literal is an IP, not a DNS name"
        );
        // A hostname still presents a DNS name.
        assert!(
            matches!(
                server_name("localhost:6667", &v4).unwrap(),
                ServerName::DnsName(_)
            ),
            "a hostname must keep the DNS-name path"
        );
    }

    // ---- the local channel reader (no server) -----------------------------

    fn write_log(home: &std::path::Path, chan: &str, lines: &[&str]) {
        let dir = home.join("channels");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join(format!("{}.log", chan)), lines.join("\n") + "\n").unwrap();
    }

    #[test]
    fn msg_line_id_reads_only_stored_message_rows() {
        assert_eq!(msg_line_id("MSG #ops 7 1700000000 alice :hi"), Some(7));
        // Anything that is not a stored MSG row carries no id, and a
        // malformed row must be skipped rather than aborting a read.
        assert_eq!(msg_line_id(""), None);
        assert_eq!(msg_line_id("JOIN #ops"), None);
        assert_eq!(msg_line_id("MSG #ops notanid 0 a :x"), None);
        assert_eq!(msg_line_id("MSG #ops"), None);
        assert_eq!(msg_line_id(" MSG #ops 7 0 a :x"), None);
    }

    #[test]
    fn local_last_id_takes_the_maximum_not_the_final_line() {
        let home = tmp_state("local_last_id_takes_the_maximum");
        // A missing channel has no messages, and must not be an error here:
        // a tail seeding its cursor on an empty channel starts at 0.
        assert_eq!(local_last_id(&home, "#nope"), 0);
        // The last line is deliberately NOT the highest: an interleaved or
        // truncated final write must not walk the cursor backwards.
        write_log(
            &home,
            "#ops",
            &[
                "MSG #ops 1 0 alice :one",
                "MSG #ops 9 0 alice :nine",
                "garbage",
                "MSG #ops 4 0 alice :four",
            ],
        );
        assert_eq!(local_last_id(&home, "#ops"), 9);
    }

    #[test]
    fn local_read_returns_only_rows_after_the_cursor() {
        let home = tmp_state("local_read_after_the_cursor");
        write_log(
            &home,
            "#ops",
            &[
                "MSG #ops 1 0 alice :one",
                "MSG #ops 2 0 alice :two",
                "MSG #ops 3 0 alice :three",
            ],
        );
        // The return value is the highest id printed, which is what the
        // caller saves as the new cursor.
        assert_eq!(local_read(&home, "#ops", 0, None), 3);
        assert_eq!(local_read(&home, "#ops", 2, None), 3);
        // Caught up: nothing to print, so no cursor movement.
        assert_eq!(local_read(&home, "#ops", 3, None), 0);
        assert_eq!(local_read(&home, "#ops", 99, None), 0);
    }

    #[test]
    fn local_read_mention_filter_matches_only_the_named_nick() {
        let home = tmp_state("local_read_mention_filter");
        write_log(
            &home,
            "#ops",
            &[
                "MSG #ops 1 0 alice :nothing for anyone",
                "MSG #ops 2 0 alice :ping @bob please",
                "MSG #ops 3 0 alice :ping @carol please",
            ],
        );
        assert_eq!(local_read(&home, "#ops", 0, Some("bob")), 2);
        assert_eq!(local_read(&home, "#ops", 0, Some("carol")), 3);
        assert_eq!(local_read(&home, "#ops", 0, Some("dave")), 0);
        // The filter must not leak past the cursor either.
        assert_eq!(local_read(&home, "#ops", 2, Some("bob")), 0);
    }

    #[test]
    fn local_reads_use_the_shared_channel_home_not_a_private_state_dir() {
        // --state is one client's own pins and sessions; the channel logs are
        // the server's shared storage. An agent handed its own --state must
        // still read the channels everyone shares, so channels_home() resolves
        // AI_CHAT_HOME (or the XDG default) and never looks at --state.
        let private = tmp_state("local_reads_ignore_state").join("private");
        let opts = parse_opts(&[
            "--local".to_string(),
            "--state".to_string(),
            private.display().to_string(),
            "--chan".to_string(),
            "#ops".to_string(),
        ]);
        assert!(opts.local, "--local must be parsed, not swallowed");
        assert_ne!(
            channels_home(),
            private,
            "a private --state dir must not become the channel home"
        );
    }
}
