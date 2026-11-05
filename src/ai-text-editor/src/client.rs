// MODE: DEV
// PACKAGE: PROD
//! Shared endpoint-resolution and autostart logic for every client of this
//! protocol. The CLI binary and the MCP adapter both need the same answer to
//! "where is my workspace, and should one be started" — this exists so they
//! call one implementation instead of maintaining two that quietly drift
//! apart. See `src/ai-text-editor/MAINTAINER.md` for the design this codifies
//! (autostart, idle shutdown, cross-file workspace reconnection, and the two
//! regressions caught while building it).

use crate::protocol::Envelope;
use crate::session;
use crate::transport::{
    endpoint_for_file, read_endpoint, read_session, request, write_session, Endpoint,
};
use stale_lock::StaleLock;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

/// What a caller already knows going in, from its own flags or arguments.
/// `method` and `file` decide, together, which of the paths below apply —
/// see `resolve`'s own comments for exactly which combination does what.
#[derive(Clone)]
pub struct ResolveRequest {
    pub file: Option<PathBuf>,
    /// T96: the `tab_id` an `open` answer reported, and addressing enough on
    /// its own. The registry knows which endpoint serves that tab and which
    /// token authorizes it, so a caller holding an id needs no path.
    pub tab_id: Option<String>,
    /// T97: a filename, or a trailing run of path components, naming a tab in
    /// this agent's workspace — the recovery for a caller that lost the id.
    /// Resolves the endpoint by identity and lets the server match the tab,
    /// which is the only side that knows what is open.
    pub tab_path: Option<String>,
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
    /// B229: "I meant this path, the missing directories and all." A path
    /// whose parent directory does not exist is refused with the parent named
    /// and a confirmation asked for; this is that confirmation, and only with
    /// it are the directories created. A silent `mkdir -p` is deliberately not
    /// the behaviour — a typo would then build a directory tree nobody asked
    /// for, which is exactly what the verify step exists to prevent. Same
    /// shape as `--acknowledge-force-save` and `--acknowledge-large-edit`.
    pub acknowledge_create_parents: bool,
    /// B241: "I have verified the recorded owner is gone; replace it." The
    /// server refuses to bind over a stale Unix endpoint whose recorded owner
    /// it cannot rule out, and its refusal names `--takeover-stale-endpoint`
    /// as the way past — a flag no client had, so the one documented recovery
    /// for capability 12 could not be performed by the tool that meets the
    /// condition. Forwarded to a server this call starts, like the other
    /// startup arguments above; it does nothing when a server is already live.
    pub takeover_stale_endpoint: bool,
    /// Skip every cached or registered session token (the endpoint may still
    /// be reused). `execute` sets it on its recovery pass, when the token on
    /// file belongs to a server generation that is gone (B179).
    pub force_refresh: bool,
}

/// What a caller sends the request to, and what to persist for next time via
/// `persist_cache`.
#[derive(Clone)]
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
/// nothing is discoverable yet.
///
/// Naming a `file` means "act on that file's tab", and opening it is part of
/// that — for every verb, not only `open` (B225). The one exception is a
/// revision-guarded verb (see [`crate::REVISION_GUARDED_METHODS`]) on a file
/// with no tab at all: the revision it carries cannot have come from a tab
/// that never existed, so it is refused by name instead of applied against a
/// guessable revision 0.
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
/// Where this identity's *focused* tab is remembered (T98).
///
/// The same file the `(identity, no file)` cache already used, which is what
/// makes the focus work with no new machinery: a request that names no file,
/// endpoint, tab id or tab path reads this entry and is served by the tab it
/// points at. What was missing was anything ever *writing* it — a successful
/// `open --file X` cached only under `(identity, X)`, so the focus was never
/// set and a bare request had nothing to resolve.
pub fn focus_path(identity: &str) -> PathBuf {
    cache_path(identity, None)
}

/// W26/AR-32/AR-37: whether `identity` already has a live focused tab --
/// the liveness half of a cold-start check. `resolve` itself never trusts a
/// cached focus without probing `Endpoint::is_live()` first (B179); a
/// cold-start check that only asked "does the cache file exist" would treat
/// a stale on-disk record naming a dead server as warm and skip the
/// autostart it actually needs. `Ok(false)` covers both "no cache file at
/// all" and "a cache file naming a dead endpoint" -- both read as
/// cold-start. `Err` surfaces a genuinely unreadable cache file rather than
/// silently folding corruption into "cold-start" (the same shape `resolve`'s
/// own cache-read error already uses).
pub fn focus_is_live(identity: &str) -> Result<bool, String> {
    let path = focus_path(identity);
    if !path.exists() {
        return Ok(false);
    }
    let session = read_session(&path)
        .map_err(|error| format!("cannot read session token {}: {error}", path.display()))?;
    Ok(session.endpoint.is_live())
}

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
        // The cache is an optimization here, never a prerequisite: a named
        // endpoint must work as the *first* call under an identity, before
        // any session file exists to read. It is also only valid for THIS
        // endpoint: `cache_path` is keyed by (identity, file), and with no
        // file it is the identity's no-file "focused tab" slot, which an
        // earlier unrelated call under the same identity may have pointed at
        // a completely different server. Forwarding that server's
        // session_token here would reconnect to its tab instead of the one
        // this call just named explicitly (B308), so the cached session is
        // only trusted when its own endpoint is the one just requested.
        let endpoint = Endpoint::parse(value);
        let session = cache_path
            .as_ref()
            .filter(|path| path.exists())
            .map(|path| read_session(path))
            .transpose()
            .map_err(|error| format!("cannot read session token: {error}"))?
            .filter(|session| session.endpoint == endpoint);
        return Ok(Resolved {
            endpoint,
            auth_token: session.as_ref().and_then(|s| s.auth_token.clone()),
            session_token: session.and_then(|s| s.session_token),
            cache_path,
        });
    }

    // T96: a tab id beats every discovery path below it, because it is the
    // most specific thing a caller can say. The registry answers with the
    // endpoint AND the tab's own session token, so an id alone is a complete
    // request — no file, no cache, no identity lookup. Its failure is fatal
    // rather than a fall-through: a caller that named a tab did not ask for
    // whichever tab discovery would have found instead, and quietly serving
    // that one is the silent mis-addressing T96-T98 exist to prevent.
    if let Some(wanted) = &request.tab_id {
        let record = session::resolve_tab(wanted)?;
        return Ok(Resolved {
            endpoint: Endpoint::parse(&record.endpoint),
            auth_token: record.auth_token.clone(),
            session_token: Some(record.session_token.clone()),
            cache_path,
        });
    }

    if let Some(path) = cache_path
        .as_ref()
        .filter(|path| path.exists() && !request.force_refresh)
    {
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
        if !request.force_refresh {
            return Err(format!(
                "explicit session token {} does not exist; provide a valid token or an endpoint",
                path.display()
            ));
        }
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
        let endpoint = Endpoint::parse(&record.endpoint);
        // A registered endpoint whose server has since died is not a live
        // workspace to reconnect to: skip the record and fall through to the
        // file path below, which can autostart a replacement that replays the
        // journal, instead of handing the caller a socket nobody listens on
        // (B179).
        if endpoint.is_live() {
            let session_token =
                if request.force_refresh || (request.method == "open" && request.file.is_some()) {
                    None
                } else {
                    Some(record.session_token.clone())
                };
            return Ok(Resolved {
                endpoint,
                auth_token: record.auth_token.clone(),
                session_token,
                cache_path,
            });
        }
    }

    let Some(file) = request.file.as_ref() else {
        // T97: with a tab path and no file, the endpoint is this agent's own
        // workspace and the server does the matching — it is the only side
        // that knows which tabs are open, and the answer to an ambiguous
        // fragment is a candidate set only it can build.
        if request.tab_path.is_some() {
            if let Some(Ok(record)) = &identity_lookup {
                let endpoint = Endpoint::parse(&record.endpoint);
                if endpoint.is_live() {
                    return Ok(Resolved {
                        endpoint,
                        auth_token: record.auth_token.clone(),
                        // Deliberately withheld: the token would route to the
                        // tab that happened to register it, before the server
                        // ever looked at `tab_path`. Same leak `open --file`
                        // withholds it for.
                        session_token: None,
                        cache_path,
                    });
                }
            }
            return Err(format!(
                "no running workspace to search for {}; `open` a file first, or name one with the request",
                request.tab_path.as_deref().unwrap_or_default()
            ));
        }
        return match identity_lookup {
            Some(Err(error)) => Err(error),
            // T98: a bare request is served by the focused tab, and this is
            // what "there is no focus" reads as. Naming the handles is the
            // difference between an agent retrying usefully and guessing.
            _ => Err(
                "no tab is focused and the request names none: `open` a file (which focuses it), or address one with tab_id, tab_path, file, or an explicit endpoint"
                    .to_owned(),
            ),
        };
    };
    let discovery = endpoint_for_file(file);
    match read_endpoint(&discovery) {
        // A discovered endpoint file outlives its server after a kill:
        // probe it, and treat a dead one as "needs a server" rather than
        // handing a socket nobody listens on to the request. A loopback TCP
        // endpoint (the Windows transport) cannot be used without its
        // secret, so carry the auth token this agent's registry records
        // know about onto the file-discovered endpoint too.
        Ok(endpoint) if endpoint.is_live() => Ok(Resolved {
            endpoint,
            auth_token: identity_lookup
                .as_ref()
                .and_then(|record| record.as_ref().ok())
                .and_then(|record| record.auth_token.clone()),
            session_token: None,
            cache_path,
        }),
        // B225: naming a file on a verb means "act on that file's tab", and
        // opening it is part of that. Only `open` used to autostart, so every
        // other verb refused a readable path it could have opened — and said
        // "no editor server is reachable" while this agent's server was alive
        // and answering other tabs, because what was actually missing was a
        // per-file discovery record, not a server (B219). An agent's instinct
        // is to name a file and ask a question about it; the client can answer
        // that instead of charging a round trip for the news.
        //
        // Both the never-opened case (`Err`, no discovery record at all) and
        // the stale-record case (`Ok` on an endpoint whose server has died)
        // now fall through to the autostart arm. Revision safety is untouched:
        // the guarded verbs below still require an explicit revision, and a
        // fresh or journal-replayed tab refuses one that does not match with
        // `stale_revision`, naming both numbers.
        //
        // The one exception is a revision-guarded verb on a file that has no
        // tab at all: the revision it carries cannot have come from a tab that
        // never existed, so opening one and applying the edit against a
        // guessable revision 0 is exactly the blind cross-buffer write the
        // guard exists to stop. That is refused by name — and only that.
        Err(_) if crate::is_revision_guarded(&request.method) => Err(format!(
            "{} carries a revision guard and {} has no tab in this workspace, so the revision it names cannot have come from one; `open` the file first and retry with the revision that open reports",
            request.method,
            file.display()
        )),
        _ => {
            let autostart_auth = autostart_server(request, file)?;
            let endpoint = read_endpoint(&discovery).map_err(|error| {
                format!(
                    "server was started but never announced an endpoint for {}: {error}",
                    file.display()
                )
            })?;
            Ok(Resolved {
                endpoint,
                auth_token: autostart_auth.or_else(|| {
                    identity_lookup
                        .as_ref()
                        .and_then(|record| record.as_ref().ok())
                        .and_then(|record| record.auth_token.clone())
                }),
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

/// `persist_cache`'s own per-(identity, file) cache, extended to `bulk_edit`:
/// every other verb's successful response carries the ONE tab's own
/// session_token, which the caller persists via `resolved.cache_path`
/// (`cache_path(identity, request.file)`) so a later standalone call for
/// that same file reconnects straight to it. `bulk_edit`'s own
/// `ResolveRequest.file` is always `None` -- its real targets live inside
/// `edits`, invisible to that generic machinery -- so without this, no
/// (identity, file) entry is EVER written for the individual files a batch
/// touches. A later standalone verb addressing one of those files by name,
/// once this identity has 2+ registered tabs, hits `session::resolve`'s own
/// `session_ambiguous` refusal and falls through to autostarting a
/// SEPARATE, duplicate server for that file -- a real resource/routing
/// hazard, not just cosmetic.
///
/// Reads the per-item `file`/`session_token` the server now echoes in
/// `bulk_edit`'s own response (`handle_bulk_edit`'s success branch). Only an
/// APPLIED item persists a cache entry, mirroring `persist_cache`'s own
/// existing caller's rule for a standalone call: a refused item's resolved
/// tab is not one this identity is authorized against, so its token must
/// never be cached as if it were.
pub fn persist_bulk_edit_item_caches(
    request: &ResolveRequest,
    resolved: &Resolved,
    auth_token: Option<&str>,
    frames: &[serde_json::Value],
) {
    if request.method != "bulk_edit" {
        return;
    }
    let Some(identity) = identity(request.explicit_identity.as_deref(), &request.agent_env_var)
    else {
        return;
    };
    let Some(results) = frames.iter().find_map(|frame| {
        frame
            .pointer("/payload/results")
            .and_then(serde_json::Value::as_array)
    }) else {
        return;
    };
    for entry in results {
        if entry.get("applied").and_then(serde_json::Value::as_bool) != Some(true) {
            continue;
        }
        let Some(file) = entry.get("file").and_then(serde_json::Value::as_str) else {
            continue;
        };
        let Some(token) = entry
            .get("session_token")
            .and_then(serde_json::Value::as_str)
        else {
            continue;
        };
        let path = cache_path(&identity, Some(Path::new(file)));
        let _ = persist_cache(Some(&path), &resolved.endpoint, auth_token, Some(token));
    }
}

/// Remember the tab this call was served by as this identity's focused tab
/// (T98), so a later request that names nothing is served by it.
///
/// Called only on a SUCCESSFUL call, and only when there is an identity to
/// focus under. A refused request must not move the focus: the tab that
/// answered a refusal is not the tab the caller meant, and focusing it would
/// send the next bare request somewhere the caller never chose — the same rule
/// that keeps a refusal from rewriting the per-(identity, file) cache.
pub fn persist_focus(
    request: &ResolveRequest,
    endpoint: &Endpoint,
    auth_token: Option<&str>,
    session_token: Option<&str>,
) {
    let Some(identity) = identity(request.explicit_identity.as_deref(), &request.agent_env_var)
    else {
        return;
    };
    // Nothing to focus on: without a tab-scoped token the entry would point at
    // an endpoint and no tab, which is what the focus is for.
    if session_token.is_none() {
        return;
    }
    let _ = write_session(&focus_path(&identity), endpoint, auth_token, session_token);
}

/// `open` with no discovered endpoint starts one itself: the agent should
/// never need to run `ai-text-editor-server start` by hand, or even know the
/// server exists as a separate process. Looks for a sibling
/// `ai-text-editor-server` next to this binary first (the installed,
/// colocated layout), falling back to `PATH`.
///
/// On Unix the replacement speaks a per-file Unix socket. Windows has no
/// usable socket here, so the autostart falls back to the loopback TCP
/// transport instead: an ephemeral port the OS assigns (`--tcp 127.0.0.1:0`)
/// and a per-start nonce the server also requires (an unauthenticated TCP
/// endpoint is refused by design). The port reaches the caller through the
/// discovery file like any other start; the secret is the returned token,
/// which `resolve` carries onto the request and the session cache. `Ok(None)`
/// means "nothing extra was needed" — the Unix transport, or a server that
/// was already alive.
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
/// Takes the whole [`ResolveRequest`] rather than one parameter per startup
/// argument: every argument here is already a field on it, the list had grown
/// to eight, and each of B217, B229 and B241 was in part a startup argument
/// that failed to reach this argv. One struct means adding the next one is an
/// edit in two places (the field and the `command.arg`), not four.
pub fn autostart_server(request: &ResolveRequest, file: &Path) -> Result<Option<String>, String> {
    let ResolveRequest {
        document_mode,
        normalize_nfc,
        idle_timeout_seconds,
        explicit_identity,
        agent_env_var,
        acknowledge_create_parents,
        takeover_stale_endpoint,
        ..
    } = request;
    let (document_mode, idle_timeout_seconds, explicit_identity) = (
        document_mode.as_deref(),
        idle_timeout_seconds.as_deref(),
        explicit_identity.as_deref(),
    );
    let (normalize_nfc, acknowledge_create_parents, takeover_stale_endpoint) = (
        *normalize_nfc,
        *acknowledge_create_parents,
        *takeover_stale_endpoint,
    );
    // B229: a path whose parent directory is missing used to be reported as
    // "server for <path> failed to start: ai-text-editor-server: cannot
    // resolve <path>: No such file or directory (os error 2)" — blaming the
    // server, when nothing about the server is wrong, and offering no
    // recovery. With the parent present that same open succeeds, the tab
    // starts empty and `save` writes the file, so the two cases differ only in
    // the parent directory and only one of them said so.
    //
    // The wording is `save`'s own for the identical condition, which the
    // server already gets right; this only reaches it from the other end.
    //
    // Two steps, not one: the refusal names the parent and asks for a
    // confirmation, and only an acknowledged retry creates the chain. A
    // silent `mkdir -p` would build a directory tree nobody asked for out of
    // a typo, which is what the verify step exists to prevent.
    if let Some(parent) = file
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        if !parent.is_dir() {
            if !acknowledge_create_parents {
                return Err(format!(
                    "cannot open {}: the parent directory {} does not exist. If the path is right, retry with acknowledge_create_parents (CLI: --acknowledge-create-parents) and the directories will be created; if it is a typo, fix the path — nothing was created",
                    file.display(),
                    parent.display()
                ));
            }
            std::fs::create_dir_all(parent).map_err(|error| {
                format!(
                    "cannot create the parent directory {} for {}: {error}",
                    parent.display(),
                    file.display()
                )
            })?;
        }
    }
    let discovery = endpoint_for_file(file);
    let lock_path = discovery.with_extension("start.lock");
    let _lock = StaleLock::acquire(&lock_path, Duration::from_secs(20))
        .map_err(|error| format!("cannot coordinate server start: {error}"))?;
    if has_live_endpoint(&discovery) {
        return Ok(None); // another `open` invocation already won this race.
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
    // Windows has no usable Unix socket, so an autostarted server there is
    // a loopback TCP endpoint on an OS-assigned ephemeral port. It also
    // refuses an unauthenticated TCP endpoint outright, so the autostart
    // mints a per-start nonce. A file (not an argv flag) carries it so the
    // secret never appears in the spawned process's command line.
    #[cfg(not(unix))]
    let (auth_file, auth_token) = {
        let file = discovery.with_extension("autostart-token");
        let token = crate::auth::nonce()
            .map_err(|error| format!("cannot create autostart auth token: {error}"))?;
        std::fs::write(&file, &token).map_err(|error| {
            format!(
                "cannot write autostart auth token {}: {error}",
                file.display()
            )
        })?;
        (file, token)
    };
    let mut command = std::process::Command::new(&binary);
    command.arg("start").arg("--file").arg(file);
    #[cfg(not(unix))]
    {
        command.arg("--tcp").arg("127.0.0.1:0");
        command.arg("--auth-token-file").arg(&auth_file);
    }
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
    // B241: the server's own gate. Without this argument reaching it, a
    // server that refuses a stale endpoint whose owner it cannot rule out
    // names a recovery the caller has no way to perform.
    if takeover_stale_endpoint {
        command.arg("--takeover-stale-endpoint");
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::from(log_capture));
    let mut child = command.spawn().map_err(|error| {
        let _ = std::fs::remove_file(&log_path);
        #[cfg(not(unix))]
        let _ = std::fs::remove_file(&auth_file);
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
            #[cfg(unix)]
            return Ok(None);
            #[cfg(not(unix))]
            return Ok(Some(auth_token));
        }
        if child.try_wait().is_ok_and(|status| status.is_some()) {
            break; // no point waiting out the deadline on an exited spawn
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let reason = std::fs::read_to_string(&log_path).unwrap_or_default();
    let _ = std::fs::remove_file(&log_path);
    #[cfg(not(unix))]
    let _ = std::fs::remove_file(&auth_file);
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

/// The request path both short-lived clients share: resolve, send, and
/// recover ONCE from the two signs that the session on file belongs to a
/// server that no longer exists — a transport failure against a cached or
/// registered endpoint, or a `session_unauthorized` refusal from a server
/// that restarted with a new generation (B179). Recovery deletes the stale
/// cache entry, re-resolves (falling through to file discovery and
/// autostart, whose journal replay makes it lossless), re-opens the named
/// file for a fresh tab-scoped token, and retries the original request.
///
/// The `build` closure turns a resolution into the request envelope, so the
/// retry carries the recovered endpoint and token rather than the stale ones.
pub fn execute<F>(
    request: &ResolveRequest,
    build: F,
) -> Result<(Vec<serde_json::Value>, Resolved), String>
where
    F: Fn(&Resolved) -> Envelope,
{
    let resolved = match resolve(request) {
        Ok(resolved) => resolved,
        Err(error) => {
            // B178: a capabilities/resources query with no server to ask is
            // answered from compiled-in defaults rather than the resolve
            // failure the cold start used to surface.
            if request.file.is_none()
                && request.explicit_endpoint.is_none()
                && matches!(request.method.as_str(), "capabilities" | "resources")
            {
                return Ok((
                    cold_discovery_frames(&request.method),
                    resolved_default(request),
                ));
            }
            return Err(error);
        }
    };
    // Every verb but `open` needs a tab-scoped session_token, and a
    // resolution that came from an autostart or from bare per-file discovery
    // carries none. Get one the way `open` does, before the request rather
    // than after the refusal: the round trip the server would spend saying
    // `session_unauthorized` buys nothing, and the recovery below then stays
    // for what it is actually for — a token that has gone stale mid-flight
    // (B225).
    let resolved =
        if request.method != "open" && request.file.is_some() && resolved.session_token.is_none() {
            refresh_token(request, &resolved).unwrap_or(resolved)
        } else {
            resolved
        };
    // Automatic revision guard: a caller of a revision-guarded verb that did
    // not supply one gets the tab's current revision fetched fresh, right
    // here, for this one call. No cache, nothing stored or reused across
    // calls -- the one-round-trip fetch-then-send gap this leaves is an
    // accepted, minimal tradeoff (ai-text-editor-automatic-revision-guard
    // plan, AR-05), not a defect to patch with a retry. A caller-supplied
    // revision (explicit, including an explicitly stale one) is always sent
    // unchanged; the server's own enforcement is completely untouched.
    let send_envelope = |resolved: &Resolved| {
        let mut envelope = build(resolved);
        if crate::is_revision_guarded(&request.method) && envelope.revision.is_none() {
            if let Some(current) = fetch_current_revision(resolved) {
                envelope.revision = Some(current);
            }
        }
        if request.method == "bulk_edit" {
            autofill_bulk_edit_session_tokens(request, resolved, &mut envelope);
        }
        envelope
    };
    let frames = request_send(&resolved, &send_envelope(&resolved))?;
    // B179's stale-token variant: a server RESTARTED on the same socket path
    // is live (so resolve did not error and the no-autostart policy is
    // untouched), but the cached tab-scoped token belongs to the dead
    // generation and the server refused the request. `open` against a live
    // server only re-routes to the file's tab and returns a fresh token — it
    // spawns nothing — so refreshing and retrying once here is in-bounds in a
    // way that autostarting a dead server is not.
    //
    // `file_mismatch` is recovered the same way and for the same reason: this
    // agent's server is alive, the token on file addresses one of its other
    // tabs, and the request named a different file. Opening the named file
    // routes to its own tab, which is what the caller asked for (B219/B225).
    // Not for a revision-guarded verb, though — a revision that came from
    // another tab must never be applied to this one, which is the whole point
    // of `ensure_tab_file`.
    let recoverable = |code: &str| match code {
        "session_unauthorized" => true,
        "file_mismatch" => !crate::is_revision_guarded(&request.method),
        _ => false,
    };
    if request.file.is_some()
        && frames.iter().any(|frame| {
            frame.get("type").and_then(serde_json::Value::as_str) == Some("error")
                && frame
                    .get("code")
                    .and_then(serde_json::Value::as_str)
                    .is_some_and(recoverable)
        })
    {
        if let Some(refreshed) = refresh_token(request, &resolved) {
            if let Ok(retried) = request_send(&resolved, &send_envelope(&refreshed)) {
                return Ok((retried, refreshed));
            }
        }
    }
    Ok((frames, resolved))
}

/// W21/AR-11/AR-23: every other verb's session_token is resolved
/// transparently by `resolve`/`cache_path` for `(identity, file)` and
/// attached to the outer envelope automatically -- `bulk_edit`'s per-item
/// `session_token` field (the server-side check W01 adds) has no client-side
/// counterpart, so an item addressing an already-open tab by `file` (the
/// exact shape the pre-existing spike tests and the validated benchmark both
/// use) would be refused with nothing the caller could have supplied. This
/// fills what the client already fills for every other verb, per item: a
/// `file`-addressed item carrying no explicit `session_token` gets this
/// identity's own cached token for that file attached before the request is
/// sent. An item naming `tab_id` needs no fill (W01's own exemption).
///
/// Deliberately not cold-start discovery itself: populating
/// `ResolveRequest.file` in the first place (so a genuinely cold call can
/// autostart or discover a server at all) is Goal 05's job in the CLI/MCP
/// entry points that build `ResolveRequest` before `execute` is ever
/// called. What this function adds on top is specific to the ONE file that
/// drove that discovery: `request.file` (Goal 05's own cold-start target,
/// when set). That file structurally cannot have a per-(identity, file)
/// cache entry yet -- discovery fell back to it precisely because none
/// existed -- but `execute`'s own pre-existing `refresh_token` step
/// (unconditional for every verb but `open`) already fetched ITS current
/// session_token into `resolved` before this closure ever ran, the exact
/// mechanism that lets a cold `replace -f new.txt` work as any identity's
/// first-ever call with no tracked token. Reusing it here, for the one
/// item whose `file` matches `request.file`, closes the gap a cold
/// `bulk_edit` would otherwise hit: the item that bootstrapped the
/// connection is not in ANY cache yet, and the server-side freshness
/// exemption (AR-06) does not cover it either, because an autostarted
/// server's own startup already opened that exact file as its default
/// tab before this request's own per-item loop ever ran -- predating the
/// batch from the per-item loop's point of view, even though it is this
/// identity's genuine first touch from the caller's.
fn autofill_bulk_edit_session_tokens(
    request: &ResolveRequest,
    resolved: &Resolved,
    envelope: &mut Envelope,
) {
    let identity = identity(request.explicit_identity.as_deref(), &request.agent_env_var);
    let Some(edits) = envelope
        .payload
        .get_mut("edits")
        .and_then(serde_json::Value::as_array_mut)
    else {
        return;
    };
    for item in edits.iter_mut() {
        let Some(object) = item.as_object_mut() else {
            continue;
        };
        if object.contains_key("tab_id") || object.contains_key("session_token") {
            continue;
        }
        let Some(file) = object
            .get("file")
            .and_then(serde_json::Value::as_str)
            .map(PathBuf::from)
        else {
            continue;
        };
        if let Some(identity) = identity.as_deref() {
            if let Ok(session) = read_session(&cache_path(identity, Some(&file))) {
                if let Some(token) = session.session_token {
                    object.insert("session_token".into(), serde_json::Value::String(token));
                    continue;
                }
            }
        }
        if request.file.as_deref() == Some(file.as_path()) {
            if let Some(token) = &resolved.session_token {
                object.insert(
                    "session_token".into(),
                    serde_json::Value::String(token.clone()),
                );
            }
        }
    }
}

/// Re-open the file against the already-live endpoint to recover a current
/// tab-scoped session token after a server restart. Never autostarts; if the
/// open itself does not answer with a token, there is nothing to refresh.
fn refresh_token(request: &ResolveRequest, resolved: &Resolved) -> Option<Resolved> {
    let file = request.file.as_ref()?;
    if let Some(path) = recovery_cache_path(request) {
        let _ = std::fs::remove_file(path);
    }
    let open_envelope = Envelope {
        version: crate::PROTOCOL_VERSION,
        request_id: "recover-open".into(),
        method: "open".into(),
        revision: None,
        auth_token: resolved.auth_token.clone(),
        session_token: None,
        payload: serde_json::json!({"file": file.to_string_lossy()}),
    };
    let frames = request_send(resolved, &open_envelope).ok()?;
    let session_token = frames.iter().find_map(|frame| {
        frame
            .pointer("/payload/session_token")
            .and_then(serde_json::Value::as_str)
            .map(str::to_owned)
    })?;
    Some(Resolved {
        endpoint: resolved.endpoint.clone(),
        auth_token: resolved.auth_token.clone(),
        session_token: Some(session_token),
        cache_path: resolved.cache_path.clone(),
    })
}

fn request_send(
    resolved: &Resolved,
    envelope: &Envelope,
) -> Result<Vec<serde_json::Value>, String> {
    request(&resolved.endpoint, envelope)
        .map_err(|error| format!("{error} (endpoint {})", resolved.endpoint.display()))
}

/// The first frame carrying a `payload.revision`, as a `u64` -- `None` if no
/// frame has one, or it is not an integer. Split out from
/// `fetch_current_revision` so it is directly unit-testable against a
/// hand-built frame with no transport involved (W17/AR-02).
fn parse_revision_from_frames(frames: &[serde_json::Value]) -> Option<u64> {
    frames.iter().find_map(|frame| {
        frame
            .pointer("/payload/revision")
            .and_then(serde_json::Value::as_u64)
    })
}

/// One-shot, uncached probe for the tab's current revision: sends a
/// `history` request (not itself revision-guarded) over the already-resolved
/// connection and reads the revision back out of the response. Never panics
/// and never propagates an error -- any failure (no server, a malformed
/// response, anything else) becomes `None`, and the caller proceeds exactly
/// as it would without this feature.
fn fetch_current_revision(resolved: &Resolved) -> Option<u64> {
    let envelope = Envelope {
        version: crate::PROTOCOL_VERSION,
        request_id: "auto-revision-fetch".into(),
        method: "history".into(),
        revision: None,
        auth_token: resolved.auth_token.clone(),
        session_token: resolved.session_token.clone(),
        payload: serde_json::json!({}),
    };
    let frames = request_send(resolved, &envelope).ok()?;
    parse_revision_from_frames(&frames)
}

fn resolved_default(request: &ResolveRequest) -> Resolved {
    let _ = request;
    Resolved {
        endpoint: Endpoint::parse("client-default"),
        auth_token: None,
        session_token: None,
        // No cache path: a cold answer has nothing real to remember, and
        // persisting this endpoint would poison the next call.
        cache_path: None,
    }
}

fn recovery_cache_path(request: &ResolveRequest) -> Option<PathBuf> {
    request.session_token_path.clone().or_else(|| {
        identity(request.explicit_identity.as_deref(), &request.agent_env_var)
            .map(|id| cache_path(&id, request.file.as_deref()))
    })
}

/// Answers for `capabilities` and `resources` when no server is reachable
/// and none was asked for: the compiled-in contract, marked as such so a
/// caller never mistakes it for a live server's configuration (B178).
pub fn cold_discovery_frames(method: &str) -> Vec<serde_json::Value> {
    let payload = match method {
        "resources" => {
            let mut report = serde_json::to_value(crate::resources::report(
                0,
                crate::DEFAULT_LARGE_THRESHOLD_BYTES,
            ))
            .unwrap_or_else(|_| serde_json::json!({}));
            if let Some(object) = report.as_object_mut() {
                object.insert("source".into(), serde_json::json!("client_default"));
            }
            report
        }
        _ => serde_json::json!({
            "protocol_version": crate::PROTOCOL_VERSION,
            "document_modes": ["text_utf8", "raw_bytes", "hex_view"],
            "search_modes": ["exact_text", "exact_bytes", "wildcard", "shell_wildcard", "path_wildcard", "regex_rust", "regex_pcre2", "fuzzy_edit", "fuzzy_subsequence", "fuzzy_token", "fuzzy_ngram", "fuzzy_phonetic", "fuzzy_soundex"],
            "presentations": ["structured", "text", "paging", "stream"],
            "defaults": {"presentation": "structured", "search_preview_matches": 4, "index_granularity": 10000, "large_file_threshold_bytes": crate::DEFAULT_LARGE_THRESHOLD_BYTES},
            "coordinates": {"text": {"line_base": 1, "column_base": 0, "column_unit": "unicode_scalar"}, "raw_bytes": {"line_base": 1, "column_base": 0, "column_unit": "byte"}, "hex_view": {"row_bytes": 16, "column_base": 0, "column_unit": "byte"}},
            "fuzzy_gradient": {"range": [0.0, 1.0], "edit": "permitted_distance_fraction", "subsequence_token_ngram": "minimum_score", "phonetic_soundex": "binary_match_score"},
            "large_file": {"bounded_reads": true, "ordinary_mutations": false, "acknowledged_job_edits": true},
            "revision_required_methods": crate::REVISION_GUARDED_METHODS,
            "transports": ["unix_socket", "loopback_tcp"],
            "source": "client_default",
            "note": "no editor server is running; these are compiled-in defaults. open a file to query a live server."
        }),
    };
    vec![
        serde_json::json!({"type": "data", "version": 1, "request_id": "cold-default", "payload": payload}),
        serde_json::json!({"type": "complete", "version": 1, "request_id": "cold-default", "sequence": 1, "result_generation": "client_default"}),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AR-01: a transport failure during the auto-fill probe must degrade to
    /// today's behavior (no auto-filled revision) rather than panic or
    /// propagate an error. Binds an ephemeral TCP port and drops the listener
    /// immediately, so the address is real but guaranteed to refuse the
    /// connection `fetch_current_revision` attempts.
    #[test]
    fn fetch_current_revision_returns_none_when_nothing_is_listening() {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind ephemeral port");
        let address = listener.local_addr().expect("local addr").to_string();
        drop(listener);
        let resolved = Resolved {
            endpoint: Endpoint::Tcp(address),
            auth_token: None,
            session_token: None,
            cache_path: None,
        };
        assert_eq!(fetch_current_revision(&resolved), None);
    }

    /// AR-02: a frame with no `payload.revision` (an error-shaped response,
    /// or any other frame missing the key) must parse to `None` rather than
    /// panic -- exercised directly, independent of any transport.
    #[test]
    fn parse_revision_from_frames_returns_none_without_a_revision_field() {
        let frames = vec![serde_json::json!({
            "type": "error",
            "version": 1,
            "request_id": "x",
            "code": "revision_required",
            "message": "missing revision",
        })];
        assert_eq!(parse_revision_from_frames(&frames), None);
    }

    /// A frame that DOES carry `payload.revision` is the positive case the
    /// two failure-path tests above need a contrast against.
    #[test]
    fn parse_revision_from_frames_finds_the_revision_when_present() {
        let frames = vec![serde_json::json!({
            "type": "data",
            "version": 1,
            "request_id": "x",
            "payload": {"revision": 7},
        })];
        assert_eq!(parse_revision_from_frames(&frames), Some(7));
    }
}
