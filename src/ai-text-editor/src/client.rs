// MODE: DEV
// PACKAGE: PROD
//! Shared endpoint-resolution and autostart logic for every client of this
//! protocol. The CLI binary and the MCP adapter both need the same answer to
//! "where is my workspace, and should one be started" — this exists so they
//! call one implementation instead of maintaining two that quietly drift
//! apart. See `src/ai-text-editor/MAINTAINER.md` for the design this codifies
//! (autostart, idle shutdown, cross-file workspace reconnection, and the two
//! regressions caught while building it).

use crate::session;
use crate::transport::{endpoint_for_file, read_endpoint, read_session, write_session, Endpoint};
use stale_lock::StaleLock;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

/// What a caller already knows going in, from its own flags or arguments.
/// `method` and `file` decide, together, which of the paths below apply —
/// see `resolve`'s own comments for exactly which combination does what.
pub struct ResolveRequest {
    pub file: Option<PathBuf>,
    pub method: String,
    /// An already-parsed `--endpoint` flag (CLI) or `arguments.endpoint`
    /// (MCP).
    pub explicit_endpoint: Option<String>,
    /// An already-parsed `--session`/`--agent` flag (CLI) or an
    /// `arguments.session`/`arguments.agent` field (MCP) — the "told
    /// directly" tier of the identity ladder. Ambient harness env vars are
    /// still considered even when this is `None`.
    pub explicit_identity: Option<String>,
    /// An explicit `--session-token PATH` (CLI only; the MCP adapter has no
    /// equivalent concept and always passes `None` here).
    pub session_token_path: Option<PathBuf>,
    /// The env var this caller's own dedicated "told directly" identity uses
    /// (`ai-text-editor` uses `TSCH_AI_EDITOR_AGENT`), independent of the
    /// harness ladder.
    pub agent_env_var: String,
    /// Forwarded to `autostart_server` if a new server ends up being
    /// started for `file`.
    pub document_mode: Option<String>,
    pub normalize_nfc: bool,
    pub idle_timeout_seconds: Option<String>,
}

/// What a caller sends the request to, and what to persist for next time via
/// `persist_cache`.
pub struct Resolved {
    pub endpoint: Endpoint,
    pub auth_token: Option<String>,
    pub session_token: Option<String>,
    /// Where to cache this resolution, if anywhere — `None` for an explicit
    /// `--endpoint` call, which has nothing of its own to remember.
    pub cache_path: Option<PathBuf>,
}

/// Resolve `explicit` (a flag/argument) or the ambient identity ladder
/// (`agent_session_key`) into one key, or `None` if nothing distinguishes
/// this agent at all — matching `KeySource::Shared`, which every caller here
/// treats the same as "no identity to reconnect by, only the file".
pub fn identity(explicit: Option<&str>, agent_env_var: &str) -> Option<String> {
    let (key, source) = agent_session_key::resolve_session_key(
        explicit,
        agent_env_var,
        &|name| std::env::var(name).ok(),
        None, // no worktree rung: identity here is per-agent, not per-checkout
    );
    match source {
        agent_session_key::KeySource::Shared => None,
        _ => Some(key),
    }
}

/// Where this identity's cached session lives, keyed by `(identity, file)`.
/// One agent with two tabs open must cache each tab's own session_token
/// separately, or a second `open` clobbers the first's cache entry and a
/// later request for the first file is silently routed to the second file's
/// tab instead — caught exactly this way while building cross-file
/// reconnection; see MAINTAINER.md.
pub fn cache_path(identity: &str, file: Option<&Path>) -> PathBuf {
    let root = std::env::var_os("TSCH_AI_EDITOR_SESSION_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("TSCH_AI_EDITOR_METADATA_DIR")
                .map(|root| PathBuf::from(root).join("sessions"))
        })
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|home| PathBuf::from(home).join(".config/tsch-ai-skills/editor/sessions"))
        })
        .unwrap_or_else(|| std::env::temp_dir().join("tsch-ai-skills-editor/sessions"));
    let material = match file {
        Some(file) => format!("{identity}:{}", file.to_string_lossy()),
        None => identity.to_string(),
    };
    let key = blake3::hash(material.as_bytes()).to_hex().to_string();
    root.join(format!("{key}.json"))
}

/// Resolve where a request should go, autostarting a server for `file` if
/// nothing is discoverable yet and `request.method == "open"`.
///
/// Precedence: an explicit `--endpoint`/`arguments.endpoint` always wins. A
/// local cache hit for `(identity, file)` reconnects to a previously-seen
/// tab directly. An explicit `--session-token PATH` that does not exist is
/// an error naming it. Otherwise this agent's own workspace registry entry
/// is tried — `session::resolve_workspace` (tolerant of several tabs sharing
/// one identity) for `open` with a file to route by, `session::resolve`
/// (demands one unambiguous tab) for everything else — and its *failure* is
/// only fatal when there is no file to fall back on: a brand-new agent with
/// nothing registered yet is the ordinary first call, not an error, and must
/// reach the file-based discovery/autostart path below rather than dying.
pub fn resolve(request: &ResolveRequest) -> Result<Resolved, String> {
    let identity = identity(request.explicit_identity.as_deref(), &request.agent_env_var);
    let cache_path = request.session_token_path.clone().or_else(|| {
        identity
            .as_deref()
            .map(|id| self::cache_path(id, request.file.as_deref()))
    });
    let identity_lookup = identity.as_deref().map(|id| {
        if request.method == "open" && request.file.is_some() {
            session::resolve_workspace(id)
        } else {
            session::resolve(id)
        }
    });

    if let Some(value) = &request.explicit_endpoint {
        let session = cache_path
            .as_ref()
            .map(|path| read_session(path))
            .transpose()
            .map_err(|error| format!("cannot read session token: {error}"))?;
        return Ok(Resolved {
            endpoint: Endpoint::parse(value),
            auth_token: session.as_ref().and_then(|s| s.auth_token.clone()),
            session_token: session.and_then(|s| s.session_token),
            cache_path,
        });
    }

    if let Some(path) = cache_path.as_ref().filter(|path| path.exists()) {
        // A previous call under this same (identity, file) — reconnects
        // straight to its own tab, skipping both the registry lookup and
        // any per-file discovery. A cached endpoint whose server has since
        // died is not a live tab to reconnect to: fall through to the file
        // path below, which can autostart a replacement that replays the
        // journal, instead of failing every later request on a socket
        // nobody is listening on.
        let session = read_session(path)
            .map_err(|error| format!("cannot read session token {}: {error}", path.display()))?;
        if session.endpoint.is_live() {
            // Same leak the registry branch withholds: the server routes by
            // session_token before it ever consults `file`, so an `open`
            // that carries this (possibly other-tab's) token reconnects to
            // the wrong tab instead of routing to the named file.
            let session_token = if request.method == "open" && request.file.is_some() {
                None
            } else {
                session.session_token
            };
            return Ok(Resolved {
                endpoint: session.endpoint,
                auth_token: session.auth_token,
                session_token,
                cache_path,
            });
        }
    }

    if let Some(path) = &request.session_token_path {
        return Err(format!(
            "explicit session token {} does not exist; provide a valid token or an endpoint",
            path.display()
        ));
    }

    if let Some(Ok(record)) = &identity_lookup {
        // The server routes a request carrying a session_token to that
        // exact tab before it ever looks at "file" in the payload — correct
        // for resuming a known tab, wrong for `open` with a file reconnecting
        // to this workspace to add or find that file specifically: forwarding
        // the *other* tab's token here would silently reconnect to that tab
        // instead of routing to the named file. Only withhold it in that one
        // case; every other call (including `open` with no file, resuming a
        // tab purely by identity) still wants the known tab.
        let session_token = if request.method == "open" && request.file.is_some() {
            None
        } else {
            Some(record.session_token.clone())
        };
        return Ok(Resolved {
            endpoint: Endpoint::parse(&record.endpoint),
            auth_token: record.auth_token.clone(),
            session_token,
            cache_path,
        });
    }

    let Some(file) = request.file.as_ref() else {
        return match identity_lookup {
            Some(Err(error)) => Err(error),
            _ => Err("a file or an endpoint is required".to_owned()),
        };
    };
    let discovery = endpoint_for_file(file);
    match read_endpoint(&discovery) {
        // A discovered endpoint file outlives its server after a kill:
        // probe it, and treat a dead one as "needs a server" rather than
        // handing a socket nobody listens on to the request.
        Ok(endpoint) if endpoint.is_live() => Ok(Resolved {
            endpoint,
            auth_token: None,
            session_token: None,
            cache_path,
        }),
        Ok(_) if request.method != "open" => Err(format!(
            "the editor server for {} has stopped; run `ai-text-editor open -f {}` to start a new one (the journal replays, but the tab is new — re-read to get a current revision and session token)",
            file.display(),
            file.display()
        )),
        Err(error) if request.method != "open" => Err(format!(
            "no server discovered for {}: {error}",
            file.display()
        )),
        _ => {
            autostart_server(
                file,
                request.document_mode.as_deref(),
                request.normalize_nfc,
                request.idle_timeout_seconds.as_deref(),
                request.explicit_identity.as_deref(),
                &request.agent_env_var,
            )?;
            let endpoint = read_endpoint(&discovery).map_err(|error| {
                format!(
                    "server was started but never announced an endpoint for {}: {error}",
                    file.display()
                )
            })?;
            Ok(Resolved {
                endpoint,
                auth_token: None,
                session_token: None,
                cache_path,
            })
        }
    }
}

/// Persist a resolution for next time, if it has a `cache_path` at all — an
/// explicit `--endpoint` call has nothing of its own to remember.
pub fn persist_cache(
    cache_path: Option<&Path>,
    endpoint: &Endpoint,
    auth_token: Option<&str>,
    session_token: Option<&str>,
) -> Result<(), String> {
    if let Some(path) = cache_path {
        write_session(path, endpoint, auth_token, session_token)
            .map_err(|error| format!("cannot save session token: {error}"))?;
    }
    Ok(())
}

/// `open` with no discovered endpoint starts one itself: the agent should
/// never need to run `ai-text-editor-server start` by hand, or even know the
/// server exists as a separate process. Looks for a sibling
/// `ai-text-editor-server` next to this binary first (the installed,
/// colocated layout), falling back to `PATH`.
///
/// Two `open` calls for the same file racing here both take this path before
/// either one has announced an endpoint. A per-file start lock (the
/// `stale-lock` primitive the session registry also uses) makes only one of
/// them actually spawn: the other waits for the lock, then finds the
/// endpoint the winner already announced. A held lock whose spawn crashed is
/// reclaimed after 20s, comfortably past the 10s an ordinary start needs to
/// announce.
///
/// `explicit_identity`/`agent_env_var`: `register_session` (server side)
/// only ever resolves its own agent_id from *its own* environment — it has
/// no argv concept of `--agent`/`--session`. An ambient harness identity
/// (`CLAUDE_CODE_SESSION_ID` and friends) reaches a spawned child
/// automatically, since `Command` inherits the parent's environment by
/// default. An *explicit* identity does not: it arrived as a flag or an MCP
/// argument, out of band from environment, so nothing about it is naturally
/// visible to the child. Without setting it here, a client resolving
/// `--agent NAME` and a server that autostarts moments later register two
/// different keys — the client's literal `NAME` and the server's own
/// harness-derived key — and every subsequent file for that name falls
/// through to yet another new server instead of reconnecting. Setting
/// `agent_env_var` on just this child process (not the caller's own
/// environment) closes that gap: the new server's own identity resolution
/// then sees the same explicit value the client already resolved.
pub fn autostart_server(
    file: &Path,
    document_mode: Option<&str>,
    normalize_nfc: bool,
    idle_timeout_seconds: Option<&str>,
    explicit_identity: Option<&str>,
    agent_env_var: &str,
) -> Result<(), String> {
    let discovery = endpoint_for_file(file);
    let lock_path = discovery.with_extension("start.lock");
    let _lock = StaleLock::acquire(&lock_path, Duration::from_secs(20))
        .map_err(|error| format!("cannot coordinate server start: {error}"))?;
    if has_live_endpoint(&discovery) {
        return Ok(()); // another `open` invocation already won this race.
    }
    let binary = server_binary_path();
    // A start that dies before announcing (a refused stale endpoint, a
    // non-UTF-8 file, a bad mode) used to vanish into Stdio::null and the
    // caller got a 10s timeout with no cause. Capture the child's stderr
    // and fold it into the failure message when the wait expires.
    let log_path = discovery.with_extension(format!("start-{}.log", std::process::id()));
    let log = std::fs::File::create(&log_path).map_err(|error| {
        format!(
            "cannot create server start log {}: {error}",
            log_path.display()
        )
    })?;
    let log_capture = log
        .try_clone()
        .map_err(|error| format!("cannot capture server start log: {error}"))?;
    let mut command = std::process::Command::new(&binary);
    command.arg("start").arg("--file").arg(file);
    if let Some(identity) = explicit_identity {
        command.env(agent_env_var, identity);
    }
    if let Some(mode) = document_mode {
        command.arg("--mode").arg(mode);
    }
    if normalize_nfc {
        command.arg("--normalize-nfc");
    }
    if let Some(timeout) = idle_timeout_seconds {
        command.arg("--idle-timeout-seconds").arg(timeout);
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::from(log_capture));
    let mut child = command.spawn().map_err(|error| {
        let _ = std::fs::remove_file(&log_path);
        format!(
            "cannot autostart {}: {error}; start it yourself with `ai-text-editor-server start --file {}`",
            binary.display(),
            file.display()
        )
    })?;
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if has_live_endpoint(&discovery) {
            let _ = std::fs::remove_file(&log_path);
            return Ok(());
        }
        if child.try_wait().is_ok_and(|status| status.is_some()) {
            break; // no point waiting out the deadline on an exited spawn
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let reason = std::fs::read_to_string(&log_path).unwrap_or_default();
    let _ = std::fs::remove_file(&log_path);
    let reason = reason.trim();
    Err(if reason.is_empty() {
        format!(
            "server for {} did not announce an endpoint in time",
            file.display()
        )
    } else {
        format!("server for {} failed to start: {reason}", file.display())
    })
}

fn has_live_endpoint(discovery: &Path) -> bool {
    read_endpoint(discovery).is_ok_and(|endpoint| endpoint.is_live())
}

fn server_binary_path() -> PathBuf {
    let name = if cfg!(windows) {
        "ai-text-editor-server.exe"
    } else {
        "ai-text-editor-server"
    };
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(name)))
        .filter(|path| path.is_file())
        .unwrap_or_else(|| PathBuf::from(name))
}
