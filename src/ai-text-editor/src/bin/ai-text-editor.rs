// MODE: DEV
// PACKAGE: PROD
use ai_text_editor::protocol::Envelope;
use ai_text_editor::session;
use ai_text_editor::transport::{
    endpoint_for_file, read_endpoint, read_session, request, write_session, Endpoint,
};
use serde_json::{json, Value};
use stale_lock::StaleLock;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::time::{Duration, Instant};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 || args.iter().any(|arg| arg == "--help" || arg == "-h") {
        help();
        return;
    }
    if args[1] == "help" {
        help();
        return;
    }
    let method = match args[1].as_str() {
        "open" => "open",
        "capabilities" => "capabilities",
        "history" => "history",
        "resources" => "resources",
        "read" => "read",
        "insert" => "insert",
        "replace" => "replace",
        "large-edit" => "large_edit",
        "begin-transaction" => "begin_transaction",
        "end-transaction" => "end_transaction",
        "restore" => "restore",
        "undo" => "undo",
        "redo" => "redo",
        "save" => "save",
        "save-as" => "save_as",
        "close" => "close",
        "resolve" => "resolve_external",
        "index" => "index",
        "cursor" => "cursor",
        "page" => "page",
        "search" => "search",
        "job-start" => "job_start",
        "job-poll" => "job_poll",
        "job-progress" => "job_progress",
        "job-complete" => "job_complete",
        "job-cancel" => "job_cancel",
        "job-transfer" => "job_transfer",
        "job-release" => "job_release",
        other => {
            eprintln!("ai-text-editor: unknown command {other}");
            std::process::exit(64);
        }
    };
    let file = option(&args, &["--file", "-f"]).map(PathBuf::from);
    let presentation =
        option(&args, &["--presentation", "-p"]).unwrap_or_else(|| "structured".into());
    let identity = session_identity(&args);
    let session_path = option(&args, &["--session-token"])
        .map(PathBuf::from)
        .or_else(|| {
            identity
                .as_deref()
                .map(|id| session_path(id, file.as_deref()))
        });
    // Resolved once, ahead of the match below: this call's own workspace
    // registry lookup, tried before ever autostarting a new server. Its
    // first miss for a brand-new agent (nothing registered yet) is entirely
    // expected and must fall through to file-based autostart, not die — the
    // fatal case is only "the caller named an identity and gave no file to
    // fall back on". `open --file X` only needs *a* reachable copy of the
    // workspace (resolve_workspace, tolerant of several tabs sharing this
    // identity); every other call needs one specific, unambiguous tab
    // (resolve), since there is no file to route by if it guesses wrong.
    let identity_lookup = identity.as_deref().map(|id| {
        if method == "open" && file.is_some() {
            session::resolve_workspace(id)
        } else {
            session::resolve(id)
        }
    });
    let (endpoint, saved_auth_token, saved_session_token) = match option(
        &args,
        &["--endpoint", "-e"],
    ) {
        Some(value) => {
            let session = session_path.as_ref().map(|path| {
                read_session(path).unwrap_or_else(|error| {
                    die(&format!(
                        "cannot read session token {}: {error}",
                        path.display()
                    ))
                })
            });
            (
                Endpoint::parse(&value),
                session
                    .as_ref()
                    .and_then(|session| session.auth_token.clone()),
                session.and_then(|session| session.session_token),
            )
        }
        None if session_path.is_some()
            && session_path.as_ref().is_some_and(|path| path.exists()) =>
        {
            // The local cache from a previous call under this same
            // (identity, file) — this is what makes a second `open` for
            // an already-known file skip both the registry lookup and
            // any per-file discovery, and reconnect straight to its own
            // tab. Keyed per file, not per identity: forwarding it is
            // always correct here, since it can only ever be the token
            // for *this* file's own tab.
            let token = session_path.as_ref().unwrap();
            let session = read_session(token).unwrap_or_else(|error| {
                die(&format!(
                    "cannot read session token {}: {error}",
                    token.display()
                ))
            });
            (session.endpoint, session.auth_token, session.session_token)
        }
        None if option(&args, &["--session-token"]).is_some() => die(&format!(
            "explicit session token {} does not exist; provide a valid token or --endpoint",
            session_path.as_ref().unwrap().display()
        )),
        None if matches!(identity_lookup, Some(Ok(_))) => {
            let Some(Ok(record)) = identity_lookup else {
                unreachable!()
            };
            // The server routes a request carrying a session_token to
            // that exact tab before it ever looks at "file" in the
            // payload — correct for resuming a known tab, wrong for
            // `open --file X` reconnecting to this workspace to add or
            // find X specifically: forwarding the *other* tab's token
            // here would silently reconnect to that tab instead of
            // routing to X. Only withhold it in that one case; every
            // other call (including `open` with no --file, resuming a
            // tab purely by identity) still wants the known tab.
            let session_token = if method == "open" && file.is_some() {
                None
            } else {
                Some(record.session_token)
            };
            (
                Endpoint::parse(&record.endpoint),
                record.auth_token,
                session_token,
            )
        }
        None => {
            let Some(file) = file.as_ref() else {
                match identity_lookup {
                    Some(Err(error)) => die(&error),
                    _ => die("--file or --endpoint is required"),
                }
            };
            match read_endpoint(&endpoint_for_file(file)) {
                Ok(endpoint) => (endpoint, None, None),
                Err(_) if method == "open" => {
                    autostart_server(file, &args);
                    let endpoint =
                            read_endpoint(&endpoint_for_file(file)).unwrap_or_else(|error| {
                                die(&format!(
                                    "server was started but never announced an endpoint for {}: {error}",
                                    file.display()
                                ))
                            });
                    (endpoint, None, None)
                }
                Err(error) => die(&format!(
                    "no server discovered for {}: {error}",
                    file.display()
                )),
            }
        }
    };
    let mut payload = serde_json::Map::new();
    if let Some(file) = &file {
        payload.insert(
            "file".into(),
            Value::String(file.to_string_lossy().into_owned()),
        );
    }
    if let Some(value) = option(&args, &["--bytes-base64"]) {
        payload.insert("bytes_base64".into(), Value::String(value));
    }
    if let Some(value) = option(&args, &["--id"]) {
        payload.insert("id".into(), json!(parse_number(&value)));
    }
    if let Some(value) = option(&args, &["--line", "-l"]) {
        payload.insert("line".into(), json!(parse_number(&value)));
    }
    if let Some(value) = option(&args, &["--column", "-c"]) {
        payload.insert("column".into(), json!(parse_number(&value)));
    }
    if let Some(value) = option(&args, &["--action", "-a"]) {
        payload.insert("action".into(), Value::String(value));
    }
    if let Some(value) = option(&args, &["--page-lines"]) {
        payload.insert("page_lines".into(), json!(parse_number(&value)));
    }
    if let Some(value) = option(&args, &["--wrap-width", "-w"]) {
        payload.insert("wrap_width".into(), json!(parse_number(&value)));
    }
    if flag(&args, &["--visual", "-V"]) {
        payload.insert("visual".into(), Value::Bool(true));
    }
    for (names, field) in [(["--before", "-b"], "before"), (["--after", "-B"], "after")] {
        if let Some(value) = option(&args, &names) {
            payload.insert(field.into(), json!(parse_number(&value)));
        }
    }
    for (argument, field) in [
        ("--range-start-line", "range_start_line"),
        ("--range-end-line", "range_end_line"),
        ("--range-start-byte", "range_start_byte"),
        ("--range-end-byte", "range_end_byte"),
    ] {
        if let Some(value) = option(&args, &[argument]) {
            payload.insert(field.into(), json!(parse_number(&value)));
        }
    }
    if let Some(value) = option(&args, &["--order"]) {
        payload.insert("order".into(), Value::String(value));
    }
    if let Some(value) = option(&args, &["--gradient", "-g"]) {
        payload.insert(
            "gradient".into(),
            json!(value
                .parse::<f64>()
                .unwrap_or_else(|_| die("--gradient must be a number"))),
        );
    }
    let expected_revision =
        option(&args, &["--expected-revision", "-r"]).map(|value| parse_number(&value));
    if let Some(value) = option(&args, &["--offset", "-o"]) {
        payload.insert("offset".into(), json!(parse_number(&value)));
    }
    if let Some(value) = option(&args, &["--length", "-L"]) {
        payload.insert("length".into(), json!(parse_number(&value)));
    }
    if let Some(value) = option(&args, &["--granularity"]) {
        payload.insert("granularity".into(), json!(parse_number(&value)));
    }
    if let Some(value) = option(&args, &["--pager-key"]) {
        payload.insert("pager_key".into(), Value::String(value));
    }
    if let Some(value) = option(&args, &["--limit", "-n"]) {
        payload.insert("limit".into(), json!(parse_number(&value)));
    }
    if let Some(value) = option(&args, &["--delete-len", "-d"]) {
        payload.insert("delete_len".into(), json!(parse_number(&value)));
    }
    if let Some(value) = option(&args, &["--text", "-t"]) {
        payload.insert("text".into(), Value::String(value));
    }
    if let Some(value) = option(&args, &["--query", "-q"]) {
        payload.insert("query".into(), Value::String(value));
    }
    if let Some(value) = option(&args, &["--query-base64"]) {
        payload.insert("query_base64".into(), Value::String(value));
    }
    if flag(&args, &["--acknowledge-force-save"]) {
        payload.insert("acknowledge_force_save".into(), Value::Bool(true));
    }
    if flag(&args, &["--acknowledge-large-edit"]) {
        payload.insert("acknowledge_large_edit".into(), Value::Bool(true));
    }
    if flag(&args, &["--preserve-external"]) {
        payload.insert("preserve_external".into(), Value::Bool(true));
    }
    if let Some(value) = option(&args, &["--backup-path"]) {
        payload.insert("backup_path".into(), Value::String(value));
    }
    if let Some(value) = option(&args, &["--target-path"]) {
        payload.insert("target_path".into(), Value::String(value));
    }
    if let Some(value) = option(&args, &["--journal-action"]) {
        payload.insert("journal_action".into(), Value::String(value));
    }
    for (argument, field) in [
        ("--job-id", "job_id"),
        ("-j", "job_id"),
        ("--cursor-id", "cursor_id"),
        ("-C", "cursor_id"),
        ("--owner", "owner"),
        ("--resume-token", "resume_token"),
    ] {
        if let Some(value) = option(&args, &[argument]) {
            if field == "job_id" || field == "cursor_id" {
                payload.insert(field.into(), json!(parse_number(&value)));
            } else {
                payload.insert(field.into(), Value::String(value));
            }
        }
    }
    if let Some(value) = option(&args, &["--progress-json"]) {
        payload.insert(
            "progress".into(),
            serde_json::from_str(&value)
                .unwrap_or_else(|error| die(&format!("invalid --progress-json: {error}"))),
        );
    }
    if let Some(value) = option(&args, &["--result-json"]) {
        payload.insert(
            "result".into(),
            serde_json::from_str(&value)
                .unwrap_or_else(|error| die(&format!("invalid --result-json: {error}"))),
        );
    }
    if flag(&args, &["--detached"]) {
        payload.insert("detached".into(), Value::Bool(true));
    }
    if flag(&args, &["--historical", "-H"]) {
        payload.insert("historical".into(), Value::Bool(true));
    }
    payload.insert("presentation".into(), Value::String(presentation.clone()));
    let auth_token = option(&args, &["--auth-token"]).or(saved_auth_token);
    if method == "search" {
        payload.insert(
            "mode".into(),
            Value::String(
                option(&args, &["--mode", "-m"])
                    .unwrap_or_else(|| die("--mode is required for search")),
            ),
        );
    }
    let envelope = Envelope {
        version: ai_text_editor::PROTOCOL_VERSION,
        request_id: "cli-1".into(),
        method: method.into(),
        revision: expected_revision,
        auth_token: auth_token.clone(),
        session_token: saved_session_token.clone(),
        payload: Value::Object(payload),
    };
    let frames = request(&endpoint, &envelope).unwrap_or_else(|error| die(&error));
    let failed = frames
        .iter()
        .any(|frame| frame.get("type").and_then(Value::as_str) == Some("error"));
    let save_path = option(&args, &["--save-session-token"])
        .map(PathBuf::from)
        .or(session_path);
    let returned_session_token = frames
        .iter()
        .find(|frame| frame.get("type").and_then(Value::as_str) == Some("data"))
        .and_then(|frame| frame.pointer("/payload/session_token"))
        .and_then(Value::as_str);
    let persisted_session_token = returned_session_token.or(saved_session_token.as_deref());
    if let Some(token) = save_path {
        write_session(
            &token,
            &endpoint,
            auth_token.as_deref(),
            persisted_session_token,
        )
        .unwrap_or_else(|error| die(&format!("cannot save session token: {error}")));
    }
    match presentation.as_str() {
        "structured" => {
            for frame in frames {
                println!("{}", serde_json::to_string_pretty(&frame).unwrap());
            }
        }
        "text" => {
            for frame in frames {
                if frame.get("type").and_then(Value::as_str) == Some("data") {
                    if let Some(text) = frame.pointer("/payload/text").and_then(Value::as_str) {
                        print!("{text}");
                    } else {
                        println!("{}", serde_json::to_string(&frame["payload"]).unwrap());
                    }
                }
            }
        }
        "paging" | "stream" => {
            for frame in frames {
                if frame.get("type").and_then(Value::as_str) == Some("data")
                    && frame.pointer("/payload/restart").and_then(Value::as_bool) == Some(true)
                {
                    if let Some(text) = frame.pointer("/payload/text").and_then(Value::as_str) {
                        print!("{text}");
                    }
                    continue;
                }
                if frame.get("type").and_then(Value::as_str) == Some("error")
                    && matches!(
                        frame.get("code").and_then(Value::as_str),
                        Some("external_change") | Some("stale_result")
                    )
                {
                    println!("===== FILE EDITED: RESTARTING =====");
                }
                if frame.get("type").and_then(Value::as_str) == Some("data") {
                    println!("{}", serde_json::to_string(&frame["payload"]).unwrap());
                } else {
                    println!("{}", serde_json::to_string(&frame).unwrap());
                }
            }
        }
        other => die(&format!(
            "unknown presentation {other}; use structured, text, paging, or stream"
        )),
    }
    if failed {
        std::process::exit(1);
    }
}

/// `open` with no discovered endpoint starts one itself: the agent should
/// never need to run `ai-text-editor-server start` by hand, or even know the
/// server exists as a separate process. Looks for a sibling
/// `ai-text-editor-server` next to this binary first (the installed,
/// colocated layout), falling back to PATH.
///
/// Two `open` calls for the same file racing here both take this path before
/// either one has announced an endpoint. A per-file start lock (the same
/// `stale-lock` primitive the session registry uses) makes only one of them
/// actually spawn: the other waits for the lock, then finds the endpoint the
/// winner already announced and returns without spawning anything itself. A
/// held lock whose spawn crashed is reclaimed after 20s, comfortably past the
/// 10s an ordinary start needs to announce.
fn autostart_server(file: &Path, args: &[String]) {
    let discovery = endpoint_for_file(file);
    let lock_path = discovery.with_extension("start.lock");
    let _lock = StaleLock::acquire(&lock_path, Duration::from_secs(20))
        .unwrap_or_else(|error| die(&format!("cannot coordinate server start: {error}")));
    if read_endpoint(&discovery).is_ok() {
        return; // another `open` invocation already won this race.
    }
    let binary = server_binary_path();
    let mut command = std::process::Command::new(&binary);
    command.arg("start").arg("--file").arg(file);
    if let Some(mode) = option(args, &["--document-mode", "-M"]) {
        command.arg("--mode").arg(mode);
    }
    if flag(args, &["--normalize-nfc"]) {
        command.arg("--normalize-nfc");
    }
    if let Some(timeout) = option(args, &["--idle-timeout-seconds"]) {
        command.arg("--idle-timeout-seconds").arg(timeout);
    }
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    command.spawn().unwrap_or_else(|error| {
        die(&format!(
            "cannot autostart {}: {error}; start it yourself with `ai-text-editor-server start --file {}`",
            binary.display(),
            file.display()
        ))
    });
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if read_endpoint(&discovery).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
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

fn option(args: &[String], names: &[&str]) -> Option<String> {
    args.windows(2)
        .find(|pair| names.contains(&pair[0].as_str()))
        .map(|pair| pair[1].clone())
}

fn flag(args: &[String], names: &[&str]) -> bool {
    args.iter().any(|arg| names.contains(&arg.as_str()))
}

/// The same explicit-id / harness-env-var precedence ladder the chat skill
/// resolves its own agent identity with (`agent-session-key`, ported out of
/// `chat-client-rs`), reused here so both tools agree on what distinguishes
/// one agent from another. `register_session` in the server resolves the
/// harness rung the same way, so a client that supplies no explicit id still
/// finds the workspace its own harness registered.
///
/// This governs which files end up sharing one server, not just literal
/// `--agent`/`--session` lookups: the server is the workspace (multiple
/// tabs, one process); the client is just the viewport onto it, the way
/// opening a file in an already-running IDE reconnects to that IDE's window
/// instead of starting a second copy. An agent that opens file A, then later
/// (possibly in a fresh, "forgetful" context with no memory of A) opens file
/// B, should land B in the *same* server A is already open in — not spin up
/// an unrelated second server keyed only by B's path. The harness rung is
/// what makes that identity persist across calls with no explicit flag.
/// `KeySource::Shared` maps to `None`: with no explicit id and no harness
/// signal at all, there is no agent identity to reconnect by, only the file.
fn session_identity(args: &[String]) -> Option<String> {
    let explicit = option(args, &["--session", "-s"]).or_else(|| option(args, &["--agent", "-A"]));
    let (key, source) = agent_session_key::resolve_session_key(
        explicit.as_deref(),
        "TSCH_AI_EDITOR_AGENT",
        &|name| std::env::var(name).ok(),
        None, // no worktree rung: identity here is per-agent, not per-checkout
    );
    match source {
        agent_session_key::KeySource::Shared => None,
        _ => Some(key),
    }
}

/// Where this identity's cached session lives. Keyed by `(identity, file)`,
/// not identity alone: one agent with two tabs open (the whole point of
/// workspace reconnection) must cache each tab's own session_token
/// separately, or the second `open` silently clobbers the first's cache
/// entry and a later `read -f a.txt` would use the endpoint but the *other*
/// file's session_token — the server would route it to that other file's
/// tab and hand back the wrong content, exactly the bug a cache-per-tab
/// exists to prevent. `file` is `None` only for the no-`--file` identity
/// resume case (`open --agent NAME` alone), which still wants one slot.
fn session_path(identity: &str, file: Option<&Path>) -> PathBuf {
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
fn parse_number(value: &str) -> u64 {
    value
        .parse()
        .unwrap_or_else(|_| die(&format!("{value} is not a non-negative integer")))
}
fn die(message: &str) -> ! {
    eprintln!("ai-text-editor: {message}");
    std::process::exit(64);
}
fn help() {
    println!("Usage: ai-text-editor COMMAND -f FILE [OPTIONS]  (or --endpoint/-e ENDPOINT for an already-open tab)");
    println!("open starts its own server when none is running yet: no separate `ai-text-editor-server start` call is needed. Use --document-mode/-M and --normalize-nfc to shape that autostart; open --endpoint ENDPOINT -f PATH adds another isolated tab to a server that is already up.");
    println!("Commands: open capabilities history resources read insert replace large-edit begin-transaction end-transaction restore undo redo save save-as close resolve index cursor page search");
    println!(
        "         job-start job-poll job-progress job-complete job-cancel job-transfer job-release"
    );
    println!("Common flags (long / short): --file -f, --endpoint -e, --line -l, --column -c, --action -a, --text -t, --query -q, --mode -m (search only), --expected-revision -r, --offset -o, --length -L, --delete-len -d, --limit -n, --cursor-id -C, --job-id -j, --presentation -p, --before -b, --after -B, --gradient -g, --wrap-width -w, --session -s, --agent -A.");
    println!("Boolean flags with a short form: --visual -V, --historical -H. Safety acknowledgements (--acknowledge-force-save, --acknowledge-large-edit) and auth/session flags are deliberately long-form only.");
    println!("Document modes: text_utf8, raw_bytes, hex_view (select at autostart with --document-mode/-M, or when starting the server yourself with --mode).");
    println!("Search requires -m/--mode and -q/--query (or --query-base64): exact_text, exact_bytes, wildcard, shell_wildcard, path_wildcard, regex_rust, regex_pcre2, fuzzy_edit, fuzzy_subsequence, fuzzy_token, fuzzy_ngram, fuzzy_phonetic, fuzzy_soundex. Fuzzy modes accept -g/--gradient 0.0..1.0 with strategy-specific defaults.");
    println!("Coordinates: text lines are 1-based and Unicode-scalar columns are 0-based; raw/hex coordinates are byte offsets. Refetch after every revision.");
    println!("Wrapped navigation: -w/--wrap-width N adds visual coordinates; -V/--visual interprets -l/-c as wrapped coordinates. Stored cursors remain logical.");
    println!("Edits: -o/--offset N or -C/--cursor-id N, plus -d/--delete-len N and -t/--text TEXT or --bytes-base64 B64; omitting -o inserts/replaces at that cursor. -r/--expected-revision N is required for safe concurrent edits. Use begin-transaction/end-transaction to group edits into one undo step.");
    println!("Reading: -b/--before N -B/--after N, -o/--offset N -L/--length N, -n/--limit N, --pager-key KEY, -H/--historical, --range-start-line N --range-end-line N, --range-start-byte N --range-end-byte N, --order forward|reverse.");
    println!("Presentation: -p/--presentation structured|text|paging|stream; paging/stream readers must restart after the FILE EDITED delimiter.");
    println!("Recovery: resolve with -a/--action backup|reload|merge|keep|force_save; backup preserves external bytes and leaves resolution pending, force-save requires --acknowledge-force-save. Add --preserve-external and optionally --backup-path PATH before discard/overwrite.");
    println!("Save-as: use save-as --target-path PATH to atomically create a new file without changing the active tab; existing targets are refused.");
    println!("Large edits: start a job, then use large-edit with -j/--job-id N --acknowledge-large-edit -o/--offset N -d/--delete-len N and replacement data; this streams and atomically replaces the file.");
    println!("Close: close first prompts with journal_close_decision_required; repeat with --journal-action preserve|clean. clean deletes the tab journal and metadata database.");
    println!("Sessions/auth: -e/--endpoint ENDPOINT, --session-token PATH, -s/--session ID, -A/--agent ID, or environment agent identity (TSCH_AI_EDITOR_AGENT, CODEX_AGENT_ID, AGENT_ID); explicit --endpoint wins, stale saved sessions are errors. --save-session-token PATH overrides automatic session storage. TCP clients use --auth-token TOKEN (or a token saved in the session file); servers accept --auth-token TOKEN or owner-only --auth-token-file PATH. Jobs use -j/--job-id N, --owner NAME, --resume-token TOKEN, --detached, --progress-json JSON, --result-json JSON.");
    println!("The server alone owns document state, history, indexes, journals, and SQLite metadata. Large files provide bounded read/index views; acknowledged large-edit jobs stream accepted rewrites and retain file-backed undo snapshots.");
}
