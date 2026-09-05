// MODE: DEV
// PACKAGE: PROD
use ai_text_editor::auth;
use ai_text_editor::document::{Document, DocumentMode};
use ai_text_editor::history::History;
use ai_text_editor::index::{LineIndex, DEFAULT_GRANULARITY};
use ai_text_editor::jobs::JobRegistry;
use ai_text_editor::journal::{Journal, JournalRecord};
use ai_text_editor::large_file::{LargeFile, LineSearchWindow};
use ai_text_editor::metadata::Metadata;
use ai_text_editor::navigation::{self, Position};
use ai_text_editor::protocol::{validate_ndjson, MAX_SERIALIZED_FRAME_BYTES};
use ai_text_editor::resources;
use ai_text_editor::revision::RevisionGuard;
use ai_text_editor::search::{find_bytes, matches_with_gradient, parse_mode, SearchMode};
use ai_text_editor::session;
#[cfg(unix)]
use ai_text_editor::transport::read_endpoint_metadata;
use ai_text_editor::transport::{
    complete, endpoint_for_file, error, error_details, response, socket_for_file, validate_request,
    write_endpoint_metadata, Endpoint,
};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};

struct Tab {
    path: PathBuf,
    document: Document,
    history: History,
    revision: u64,
    metadata: Metadata,
    disk_digest: String,
    base_bytes: Vec<u8>,
    pending_external: Option<Vec<u8>>,
    index: LineIndex,
    index_loaded: bool,
    index_complete: bool,
    cursors: BTreeMap<u64, Position>,
    results: HashMap<String, Vec<Value>>,
    journal: Journal,
    journal_seq: u64,
    auth_token: Option<String>,
    session_token: String,
    server_generation: String,
    large_file: Option<LargeFile>,
    large_threshold_bytes: u64,
    jobs: JobRegistry,
    close_after_response: bool,
    transaction_before: Option<Document>,
    large_undo: Vec<LargeHistory>,
    large_redo: Vec<LargeHistory>,
    /// Digest of the buffer as of the last save/load/reload. `dirty` means
    /// the buffer moved away from this; `disk_diverged` means the file on
    /// disk moved away from what the tab last synced (B183's split).
    saved_digest: String,
    /// Journal edits replayed when this tab opened; reported by `open` so a
    /// recovered revision is never mistaken for fresh work (B196).
    replayed_edits: usize,
}

struct ServerState {
    tabs: HashMap<String, Arc<Mutex<Tab>>>,
    default_key: String,
    mode: DocumentMode,
    normalize_nfc: bool,
    auth_token: Option<String>,
    server_generation: String,
    large_threshold_bytes: u64,
    endpoint: Option<Endpoint>,
    discovery_path: PathBuf,
    last_activity: std::time::Instant,
    /// Requests currently being handled, across every connection. A long
    /// synchronous operation — a large-file edit, an unbounded-feeling
    /// search — holds this above zero for its whole duration, so idle means
    /// no request in flight, not merely no request having *arrived* lately.
    in_flight: u64,
}

#[derive(Debug, Clone)]
struct LargeHistory {
    before: PathBuf,
    after: PathBuf,
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("Usage: ai-text-editor-server start --file PATH [--mode text_utf8|raw_bytes|hex_view] [--normalize-nfc] [--idle-timeout-seconds N] [--takeover-stale-endpoint] [--tcp HOST:PORT --auth-token TOKEN|--auth-token-file PATH]");
        println!("The server owns the initial tab and accepts one newline-delimited JSON request per connection. Use client open --endpoint ENDPOINT --file PATH to add isolated tabs; each tab has its own session token and metadata. Files above 256 MiB use bounded access; override with --large-threshold-bytes. A stale Unix endpoint is retained and requires --takeover-stale-endpoint for explicit replacement.");
        println!("This binary is not meant to be invoked directly by an agent: the ai-text-editor client's `open` command starts one itself when none is running. --idle-timeout-seconds sets how long the server waits with no request before exiting on its own (default 600, 0 disables); the next open replays the journal and picks up where it left off.");
        return;
    }
    if args.get(1).map(String::as_str) != Some("start") {
        eprintln!("usage: ai-text-editor-server start --file PATH [--mode text_utf8|raw_bytes|hex_view] [--takeover-stale-endpoint] [--tcp HOST:PORT --auth-token TOKEN|--auth-token-file PATH]");
        std::process::exit(64);
    }
    let path = match option(&args, "--file") {
        Some(path) => ai_text_editor::transport::canonical_or_near(Path::new(&path))
            .unwrap_or_else(|error| die(&format!("cannot resolve {path}: {error}"))),
        None => die("--file is required"),
    };
    let auth_token_file = option(&args, "--auth-token-file").map(PathBuf::from);
    if auth_token_file.is_some() && option(&args, "--auth-token").is_some() {
        die("--auth-token and --auth-token-file are mutually exclusive");
    }
    let configured_auth_token = auth_token_file
        .as_deref()
        .map(read_auth_token_file)
        .transpose()
        .unwrap_or_else(|error| die(&format!("cannot read --auth-token-file: {error}")))
        .or_else(|| option(&args, "--auth-token"));
    let session_token =
        auth::nonce().unwrap_or_else(|error| die(&format!("cannot create session token: {error}")));
    let server_generation = server_generation();
    let file_size = match fs::metadata(&path) {
        Ok(metadata) => metadata.len(),
        // A file that does not exist yet opens as an empty document; it is
        // created at its real path by the first save.
        Err(error) if error.kind() == io::ErrorKind::NotFound => 0,
        Err(error) => die(&format!("cannot stat {}: {error}", path.display())),
    };
    let large_threshold = option(&args, "--large-threshold-bytes")
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or_else(|| {
            resources::report(0, 256 * 1024 * 1024)
                .available_memory_bytes
                .map(|available| (available / 4).clamp(64 * 1024 * 1024, 256 * 1024 * 1024))
                .unwrap_or(256 * 1024 * 1024)
        });
    let large_file = (file_size > large_threshold).then(|| {
        LargeFile::open(&path)
            .unwrap_or_else(|error| die(&format!("cannot open large file: {error}")))
    });
    let bytes = if large_file.is_some() {
        Vec::new()
    } else {
        match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(error) => die(&format!("cannot read {}: {error}", path.display())),
        }
    };
    let mode = match option(&args, "--mode").as_deref() {
        None | Some("text_utf8") => DocumentMode::TextUtf8,
        Some("raw_bytes") => DocumentMode::RawBytes,
        Some("hex_view") => DocumentMode::HexView,
        Some(value) => die(&format!(
            "unknown mode {value}; use text_utf8, raw_bytes, or hex_view"
        )),
    };
    let mut document = match Document::new(bytes, mode) {
        Ok(document) => document,
        Err(_) => die("file is not UTF-8; restart with --mode raw_bytes or --mode hex_view"),
    };
    if args.iter().any(|arg| arg == "--normalize-nfc") {
        document
            .enable_nfc()
            .unwrap_or_else(|error| die(&format!("cannot enable NFC normalization: {error}")));
    }
    let metadata = Metadata::open(&path)
        .unwrap_or_else(|error| die(&format!("cannot open tab metadata: {error}")));
    let disk_digest = disk_state(&path, large_file.as_ref(), document.bytes());
    let saved_digest = digest(document.bytes());
    let journal_root = std::env::var_os("TSCH_AI_EDITOR_METADATA_DIR")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|home| PathBuf::from(home).join(".config/tsch-ai-skills/editor"))
        })
        .or_else(|| {
            std::env::var_os("USERPROFILE")
                .map(|home| PathBuf::from(home).join("codex/tsch-ai-skills/editor"))
        })
        .unwrap_or_else(|| std::env::temp_dir().join("tsch-ai-skills/editor"));
    fs::create_dir_all(&journal_root)
        .unwrap_or_else(|error| die(&format!("cannot create journal directory: {error}")));
    let identity = fs::canonicalize(&path).unwrap_or_else(|_| path.clone());
    let journal_path = journal_root.join(format!(
        "journal-{}.ndjson",
        blake3::hash(identity.to_string_lossy().as_bytes()).to_hex()
    ));
    let mut journal = Journal::open(journal_path)
        .unwrap_or_else(|error| die(&format!("cannot open journal: {error}")));
    let records = journal
        .replay()
        .unwrap_or_else(|error| die(&format!("cannot recover journal: {error}")));
    let replayed_edits = records
        .iter()
        .filter(|record| matches!(record.kind.as_str(), "edit" | "large_edit" | "restore"))
        .count();
    let mut history = History::default();
    let mut recovered_revision = 0;
    let mut journal_seq = 0;
    let mut large_undo = Vec::new();
    let mut large_redo = Vec::new();
    for record in records {
        journal_seq = journal_seq.max(record.seq);
        recovered_revision = recovered_revision.max(
            record
                .payload
                .get("revision")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        );
        let (Some(before), Some(after)) = (
            record.payload.get("before").and_then(Value::as_str),
            record.payload.get("after").and_then(Value::as_str),
        ) else {
            if matches!(
                record.kind.as_str(),
                "large_edit" | "large_undo" | "large_redo"
            ) {
                if let (Some(before), Some(after)) = (
                    record.payload.get("before_path").and_then(Value::as_str),
                    record.payload.get("after_path").and_then(Value::as_str),
                ) {
                    let before = PathBuf::from(before);
                    let after = PathBuf::from(after);
                    if before.is_file() && after.is_file() {
                        let snapshot = LargeHistory { before, after };
                        if record.kind == "large_edit" {
                            large_undo.push(snapshot);
                        } else if record.kind == "large_undo" {
                            if let Some(previous) = large_undo.pop() {
                                large_redo.push(previous);
                            }
                        } else if record.kind == "large_redo" {
                            if let Some(previous) = large_redo.pop() {
                                large_undo.push(previous);
                            }
                        }
                    }
                }
            }
            continue;
        };
        let decode = |value: &str| {
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, value)
                .map_err(|error| error.to_string())
        };
        let before = decode(before)
            .unwrap_or_else(|error| die(&format!("cannot recover journal snapshot: {error}")));
        let after = decode(after)
            .unwrap_or_else(|error| die(&format!("cannot recover journal snapshot: {error}")));
        let before_document = Document::new(before, mode)
            .unwrap_or_else(|error| die(&format!("journal snapshot is invalid: {error}")));
        let after_document = Document::new(after, mode)
            .unwrap_or_else(|error| die(&format!("journal snapshot is invalid: {error}")));
        document = after_document.clone();
        history.record(&before_document, &after_document);
    }
    let rebuilt_index = if let Some(file) = &large_file {
        file.index_prefix(DEFAULT_GRANULARITY, DEFAULT_GRANULARITY)
            .unwrap_or_else(|_| LineIndex::build(document.bytes(), DEFAULT_GRANULARITY))
    } else {
        LineIndex::build(document.bytes(), DEFAULT_GRANULARITY)
    };
    let (index, loaded_index) = match metadata.load_index(
        DEFAULT_GRANULARITY,
        rebuilt_index.bytes,
        &disk_digest,
        recovered_revision,
    ) {
        Ok(Some(index)) => (index, true),
        _ => (rebuilt_index, false),
    };
    let _ = metadata.record(&path, mode, recovered_revision, index.bytes);
    let index_complete = loaded_index || large_file.is_none();
    let mut tab = Tab {
        path: path.clone(),
        document,
        history,
        revision: recovered_revision,
        metadata,
        disk_digest,
        base_bytes: if large_file.is_some() {
            Vec::new()
        } else {
            tab_base_bytes(&path)
        },
        pending_external: None,
        index,
        index_loaded: loaded_index,
        index_complete,
        cursors: BTreeMap::from([(0, Position { line: 1, column: 0 })]),
        results: HashMap::new(),
        journal,
        journal_seq,
        auth_token: configured_auth_token.clone(),
        session_token: session_token.clone(),
        server_generation: server_generation.clone(),
        large_file,
        large_threshold_bytes: large_threshold,
        jobs: JobRegistry::default(),
        close_after_response: false,
        transaction_before: None,
        large_undo,
        large_redo,
        saved_digest,
        replayed_edits,
    };
    if !loaded_index {
        persist_index(&mut tab);
    }
    let tab = Arc::new(Mutex::new(tab));
    let default_key = tab_key(&path);
    let state = Arc::new(Mutex::new(ServerState {
        tabs: HashMap::from([(default_key.clone(), Arc::clone(&tab))]),
        default_key,
        mode,
        normalize_nfc: args.iter().any(|arg| arg == "--normalize-nfc"),
        auth_token: configured_auth_token.clone(),
        server_generation: server_generation.clone(),
        large_threshold_bytes: large_threshold,
        endpoint: None,
        discovery_path: path.clone(),
        last_activity: std::time::Instant::now(),
        in_flight: 0,
    }));
    let idle_timeout = option(&args, "--idle-timeout-seconds")
        .map(|value| {
            value
                .parse::<u64>()
                .unwrap_or_else(|_| die("--idle-timeout-seconds must be a non-negative integer"))
        })
        .map(Duration::from_secs)
        .unwrap_or(Duration::from_secs(600));
    if !idle_timeout.is_zero() {
        spawn_idle_watchdog(Arc::clone(&state), idle_timeout);
    }
    let requested_tcp = option(&args, "--tcp");
    if let Some(address) = requested_tcp {
        let auth_token = configured_auth_token.unwrap_or_default();
        if auth_token.is_empty() {
            die("--auth-token or --auth-token-file is required with --tcp; refusing an unauthenticated TCP endpoint");
        }
        let listener = TcpListener::bind(&address)
            .unwrap_or_else(|error| die(&format!("cannot bind {address}: {error}")));
        let endpoint = Endpoint::Tcp(listener.local_addr().unwrap().to_string());
        if let Ok(mut state_guard) = state.lock() {
            state_guard.endpoint = Some(endpoint.clone());
            state_guard.auth_token = Some(auth_token.clone());
        }
        announce(&path, &endpoint, &server_generation);
        register_session(
            &endpoint,
            &server_generation,
            &session_token,
            Some(&auth_token),
        );
        let generation = server_generation;
        for stream in listener.incoming().flatten() {
            let secret = if let Some(path) = auth_token_file.as_deref() {
                match read_auth_token_file(path) {
                    Ok(secret) => secret,
                    Err(error) => {
                        eprintln!(
                            "ai-text-editor-server: cannot reread authentication file: {error}"
                        );
                        continue;
                    }
                }
            } else {
                auth_token.clone()
            };
            if secret.is_empty() {
                eprintln!(
                    "ai-text-editor-server: authentication file is empty; refusing connection"
                );
                continue;
            }
            if let Ok(state_guard) = state.lock() {
                for tab in state_guard.tabs.values() {
                    if let Ok(mut tab) = tab.lock() {
                        tab.auth_token = Some(secret.clone());
                    }
                }
            }
            let state = Arc::clone(&state);
            let generation = generation.clone();
            std::thread::spawn(move || {
                serve_tcp(stream, state, secret.as_bytes(), &generation);
            });
        }
        return;
    }
    #[cfg(unix)]
    {
        let socket = socket_for_file(&path);
        if let Some(parent) = socket.parent() {
            fs::create_dir_all(parent)
                .unwrap_or_else(|error| die(&format!("cannot create socket directory: {error}")));
        }
        if socket.exists() {
            if UnixStream::connect(&socket).is_ok() {
                die(&format!(
                    "an editor server already owns {}",
                    socket.display()
                ));
            }
            let discovery = endpoint_for_file(&path);
            if !args.iter().any(|arg| arg == "--takeover-stale-endpoint")
                && !endpoint_owner_is_gone(&discovery)
            {
                let details = read_endpoint_metadata(&discovery)
                    .ok()
                    .map(|metadata| {
                        format!(
                            " (pid {}, generation {})",
                            metadata
                                .pid
                                .map(|pid| pid.to_string())
                                .unwrap_or_else(|| "unknown".into()),
                            metadata.generation.as_deref().unwrap_or("unknown")
                        )
                    })
                    .unwrap_or_default();
                die(&format!(
                    "stale editor endpoint detected at {}{}; refusing to impersonate it; retry with --takeover-stale-endpoint after verifying ownership",
                    discovery.display(), details
                ));
            }
            if discovery.exists() {
                let stale = discovery.with_extension(format!("stale-{}", server_generation));
                fs::rename(&discovery, &stale).unwrap_or_else(|error| {
                    die(&format!(
                        "cannot preserve stale endpoint {}: {error}",
                        discovery.display()
                    ))
                });
            }
            fs::remove_file(&socket)
                .unwrap_or_else(|error| die(&format!("cannot remove stale socket: {error}")));
        }
        let listener = UnixListener::bind(&socket)
            .unwrap_or_else(|error| die(&format!("cannot bind {}: {error}", socket.display())));
        let endpoint = Endpoint::Unix(socket.clone());
        if let Ok(mut state_guard) = state.lock() {
            state_guard.endpoint = Some(endpoint.clone());
        }
        announce(&path, &endpoint, &server_generation);
        register_session(
            &endpoint,
            &server_generation,
            &session_token,
            configured_auth_token.as_deref(),
        );
        for stream in listener.incoming().flatten() {
            let state = Arc::clone(&state);
            std::thread::spawn(move || serve(stream, state));
        }
    }
    #[cfg(not(unix))]
    die("Unix sockets are unavailable; provide --tcp HOST:PORT");
}

/// The server is meant to be invisible to the agent driving it: it starts
/// itself on the first `open` and, symmetrically, stops itself once nobody
/// has asked it anything for `idle_timeout` — the next `open` for this file
/// just starts another one, replaying the journal it left behind.
fn spawn_idle_watchdog(state: Arc<Mutex<ServerState>>, idle_timeout: Duration) {
    let poll_interval = idle_timeout.min(Duration::from_secs(15));
    std::thread::spawn(move || loop {
        std::thread::sleep(poll_interval);
        let (idle_for, in_flight, discovery_path, tabs) = {
            let guard = state.lock().unwrap();
            (
                guard.last_activity.elapsed(),
                guard.in_flight,
                guard.discovery_path.clone(),
                guard.tabs.values().cloned().collect::<Vec<_>>(),
            )
        };
        // A request still being handled (a large-file edit, a search with
        // no natural stopping point) keeps in_flight above zero for its
        // whole duration; a detached job — the same large-edit machinery,
        // resumed later by job-poll — can be active with no connection open
        // at all. Either one means the server is working, not idle, no
        // matter how long since a request last arrived.
        if idle_for < idle_timeout || in_flight > 0 {
            continue;
        }
        let any_job_active = tabs
            .iter()
            .any(|tab| tab.lock().is_ok_and(|tab| tab.jobs.has_active()));
        if any_job_active {
            continue;
        }
        let _ = fs::remove_file(endpoint_for_file(&discovery_path));
        let _ = fs::remove_file(socket_for_file(&discovery_path));
        std::process::exit(0);
    });
}

fn register_session(
    endpoint: &Endpoint,
    server_generation: &str,
    session_token: &str,
    auth_token: Option<&str>,
) {
    // Same ladder the client's `session_identity` resolves with (no
    // explicit flag on this side — the server has no argv concept of
    // "--agent", only its own environment), so a client that supplies no
    // explicit id still finds the tab its own harness registered.
    let (key, source) = agent_session_key::resolve_session_key(
        None,
        "TSCH_AI_EDITOR_AGENT",
        &|name| std::env::var(name).ok(),
        None,
    );
    let agent_id = match source {
        agent_session_key::KeySource::Shared => None,
        _ => Some(key),
    };
    let record = session::new_record(
        &endpoint.display(),
        server_generation,
        session_token,
        auth_token,
        agent_id,
    );
    if let Err(error) = session::register(&record) {
        eprintln!("ai-text-editor-server: cannot register session: {error}");
    }
}

fn persist_index(tab: &mut Tab) {
    let _ = tab.metadata.record_index(
        tab.index.granularity,
        tab.index.bytes,
        tab.index.lines,
        &tab.disk_digest,
        tab.index_complete,
        tab.revision,
    );
    let _ = tab.metadata.record_index_blocks(
        &tab.index.blocks,
        tab.index.granularity,
        &tab.disk_digest,
        tab.revision,
    );
}

fn open_additional_tab(
    path: PathBuf,
    mode: DocumentMode,
    normalize_nfc: bool,
    auth_token: Option<String>,
    server_generation: String,
    large_threshold_bytes: u64,
) -> Result<Tab, String> {
    let file_size = match fs::metadata(&path) {
        Ok(metadata) => metadata.len(),
        Err(error) if error.kind() == io::ErrorKind::NotFound => 0,
        Err(error) => return Err(format!("cannot stat {}: {error}", path.display())),
    };
    let large_file = if file_size > large_threshold_bytes {
        Some(LargeFile::open(&path).map_err(|error| format!("cannot open large file: {error}"))?)
    } else {
        None
    };
    let bytes = if large_file.is_some() {
        Vec::new()
    } else {
        match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(error) => return Err(format!("cannot read {}: {error}", path.display())),
        }
    };
    let mut document = Document::new(bytes, mode)
        .map_err(|_| "file is not UTF-8; open it with raw_bytes or hex_view mode".to_owned())?;
    if normalize_nfc {
        document
            .enable_nfc()
            .map_err(|error| format!("cannot enable NFC normalization: {error}"))?;
    }
    let metadata =
        Metadata::open(&path).map_err(|error| format!("cannot open tab metadata: {error}"))?;
    let disk_digest = disk_state(&path, large_file.as_ref(), document.bytes());
    let saved_digest = digest(document.bytes());
    let journal_root = session::metadata_root();
    fs::create_dir_all(&journal_root)
        .map_err(|error| format!("cannot create journal directory: {error}"))?;
    let journal_path = journal_root.join(format!(
        "journal-{}.ndjson",
        blake3::hash(path.to_string_lossy().as_bytes()).to_hex()
    ));
    let mut journal =
        Journal::open(journal_path).map_err(|error| format!("cannot open journal: {error}"))?;
    let records = journal
        .replay()
        .map_err(|error| format!("cannot recover journal: {error}"))?;
    let replayed_edits = records
        .iter()
        .filter(|record| matches!(record.kind.as_str(), "edit" | "large_edit" | "restore"))
        .count();
    let mut history = History::default();
    let mut recovered_revision = 0;
    let mut journal_seq = 0;
    let mut large_undo = Vec::new();
    let mut large_redo = Vec::new();
    for record in records {
        journal_seq = journal_seq.max(record.seq);
        recovered_revision = recovered_revision.max(
            record
                .payload
                .get("revision")
                .and_then(Value::as_u64)
                .unwrap_or(0),
        );
        let (Some(before), Some(after)) = (
            record.payload.get("before").and_then(Value::as_str),
            record.payload.get("after").and_then(Value::as_str),
        ) else {
            if matches!(
                record.kind.as_str(),
                "large_edit" | "large_undo" | "large_redo"
            ) {
                if let (Some(before), Some(after)) = (
                    record.payload.get("before_path").and_then(Value::as_str),
                    record.payload.get("after_path").and_then(Value::as_str),
                ) {
                    let snapshot = LargeHistory {
                        before: PathBuf::from(before),
                        after: PathBuf::from(after),
                    };
                    if snapshot.before.is_file() && snapshot.after.is_file() {
                        if record.kind == "large_edit" {
                            large_undo.push(snapshot);
                        } else if record.kind == "large_undo" {
                            if let Some(previous) = large_undo.pop() {
                                large_redo.push(previous);
                            }
                        } else if record.kind == "large_redo" {
                            if let Some(previous) = large_redo.pop() {
                                large_undo.push(previous);
                            }
                        }
                    }
                }
            }
            continue;
        };
        let decode = |value: &str| {
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, value)
                .map_err(|error| error.to_string())
        };
        let before =
            decode(before).map_err(|error| format!("cannot recover journal snapshot: {error}"))?;
        let after =
            decode(after).map_err(|error| format!("cannot recover journal snapshot: {error}"))?;
        let before_document = Document::new(before, mode)
            .map_err(|error| format!("journal snapshot is invalid: {error}"))?;
        let after_document = Document::new(after, mode)
            .map_err(|error| format!("journal snapshot is invalid: {error}"))?;
        document = after_document.clone();
        history.record(&before_document, &after_document);
    }
    let rebuilt_index = if let Some(file) = &large_file {
        file.index_prefix(DEFAULT_GRANULARITY, DEFAULT_GRANULARITY)
            .unwrap_or_else(|_| LineIndex::build(document.bytes(), DEFAULT_GRANULARITY))
    } else {
        LineIndex::build(document.bytes(), DEFAULT_GRANULARITY)
    };
    let (index, index_loaded) = match metadata.load_index(
        DEFAULT_GRANULARITY,
        rebuilt_index.bytes,
        &disk_digest,
        recovered_revision,
    ) {
        Ok(Some(index)) => (index, true),
        _ => (rebuilt_index, false),
    };
    let _ = metadata.record(&path, mode, recovered_revision, index.bytes);
    let session_token =
        auth::nonce().map_err(|error| format!("cannot create session token: {error}"))?;
    let index_complete = index_loaded || large_file.is_none();
    let mut tab = Tab {
        path: path.clone(),
        document,
        history,
        revision: recovered_revision,
        metadata,
        disk_digest,
        base_bytes: if large_file.is_some() {
            Vec::new()
        } else {
            tab_base_bytes(&path)
        },
        pending_external: None,
        index,
        index_loaded,
        index_complete,
        cursors: BTreeMap::from([(0, Position { line: 1, column: 0 })]),
        results: HashMap::new(),
        journal,
        journal_seq,
        auth_token,
        session_token,
        server_generation,
        large_file,
        large_threshold_bytes,
        jobs: JobRegistry::default(),
        close_after_response: false,
        transaction_before: None,
        large_undo,
        large_redo,
        saved_digest,
        replayed_edits,
    };
    if !index_loaded {
        persist_index(&mut tab);
    }
    Ok(tab)
}

fn option(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
}

fn read_auth_token_file(path: &std::path::Path) -> io::Result<String> {
    #[cfg(unix)]
    {
        let metadata = fs::metadata(path)?;
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o077 != 0 {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "authentication file must not be group- or world-accessible",
            ));
        }
    }
    let token = fs::read_to_string(path)?.trim().to_owned();
    if token.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "authentication file is empty",
        ));
    }
    Ok(token)
}

fn announce(path: &PathBuf, endpoint: &Endpoint, generation: &str) {
    let discovery = endpoint_for_file(path);
    write_endpoint_metadata(&discovery, endpoint, std::process::id(), generation).unwrap_or_else(
        |error| {
            die(&format!(
                "cannot publish endpoint {}: {error}",
                discovery.display()
            ))
        },
    );
    println!(
        "{}",
        json!({"file": path, "endpoint": endpoint.display(), "discovery": discovery})
    );
    let _ = std::io::stdout().flush();
}

/// Marks the server busy for as long as it lives, and stamps `last_activity`
/// again on drop — so the idle watchdog measures time with no request
/// in flight, not merely time since the last one arrived. Held across the
/// whole connection, not just the JSON dispatch, so a slow read of a large
/// request body counts as busy too.
struct BusyGuard<'a> {
    state: &'a Arc<Mutex<ServerState>>,
}

fn mark_busy(state: &Arc<Mutex<ServerState>>) -> BusyGuard<'_> {
    if let Ok(mut guard) = state.lock() {
        guard.in_flight += 1;
    }
    BusyGuard { state }
}

impl Drop for BusyGuard<'_> {
    fn drop(&mut self) {
        if let Ok(mut guard) = self.state.lock() {
            guard.in_flight = guard.in_flight.saturating_sub(1);
            guard.last_activity = std::time::Instant::now();
        }
    }
}

#[cfg(unix)]
fn serve<S: std::io::Read + std::io::Write>(stream: S, state: Arc<Mutex<ServerState>>) {
    let _busy = mark_busy(&state);
    let mut reader = BufReader::new(stream);
    let mut line = Vec::new();
    if reader.read_until(b'\n', &mut line).unwrap_or(0) == 0 {
        return;
    }
    let request_id = "unknown";
    let writer = reader.get_mut();
    let default_tab = default_tab(&state);
    let (value, selected_tab) = match validate_ndjson(&line).and_then(validate_request) {
        Ok(envelope) => match select_tab(&envelope, &state) {
            Ok(selected) => (handle(envelope, &selected), selected),
            Err(message) => (
                vec![error(
                    &envelope.request_id,
                    if message.starts_with("session_unauthorized:") {
                        "session_unauthorized"
                    } else {
                        "tab_open_failed"
                    },
                    message,
                )],
                default_tab.clone(),
            ),
        },
        Err(error_value) => (
            vec![error(
                request_id,
                "invalid_request",
                error_value.to_string(),
            )],
            default_tab,
        ),
    };
    write_frames(writer, value, &selected_tab, &state);
}

fn serve_tcp(
    mut stream: std::net::TcpStream,
    state: Arc<Mutex<ServerState>>,
    secret: &[u8],
    generation: &str,
) {
    let _busy = mark_busy(&state);
    let encoded_nonce = match auth::nonce() {
        Ok(nonce) => nonce,
        Err(error_value) => {
            eprintln!(
                "ai-text-editor-server: cannot create authentication challenge: {error_value}"
            );
            return;
        }
    };
    let challenge_nonce = match auth::decode_nonce(&encoded_nonce) {
        Ok(nonce) => nonce,
        Err(error_value) => {
            eprintln!(
                "ai-text-editor-server: cannot decode authentication challenge: {error_value}"
            );
            return;
        }
    };
    let challenge = json!({
        "version": ai_text_editor::PROTOCOL_VERSION,
        "request_id": "auth",
        "type": "challenge",
        "payload": {"nonce": encoded_nonce, "generation": generation}
    });
    let mut challenge_bytes = serde_json::to_vec(&challenge).unwrap_or_default();
    challenge_bytes.push(b'\n');
    if stream.write_all(&challenge_bytes).is_err() || stream.flush().is_err() {
        return;
    }
    let mut reader = BufReader::new(stream);
    let mut line = Vec::new();
    if reader.read_until(b'\n', &mut line).unwrap_or(0) == 0 {
        return;
    }
    let authenticated = validate_ndjson(&line)
        .and_then(validate_request)
        .ok()
        .filter(|envelope| envelope.method == "authenticate")
        .and_then(|envelope| {
            let nonce_bytes =
                auth::decode_nonce(envelope.payload.get("nonce").and_then(Value::as_str)?).ok()?;
            if nonce_bytes != challenge_nonce {
                return None;
            }
            let proof = envelope.payload.get("proof").and_then(Value::as_str)?;
            auth::verify(
                secret,
                &nonce_bytes,
                &envelope.request_id,
                generation,
                proof,
            )
            .then_some(envelope.request_id)
        });
    let Some(request_id) = authenticated else {
        let writer = reader.get_mut();
        let tab = default_tab(&state);
        write_frames(
            writer,
            vec![error(
                "unknown",
                "authentication_failed",
                "invalid TCP challenge proof",
            )],
            &tab,
            &state,
        );
        return;
    };
    line.clear();
    if reader.read_until(b'\n', &mut line).unwrap_or(0) == 0 {
        return;
    }
    let default_tab = default_tab(&state);
    let (value, selected_tab) = match validate_ndjson(&line).and_then(validate_request) {
        Ok(mut envelope) if envelope.request_id == request_id => {
            // Authentication is bound to this connection and request id. Do
            // not require the secret to cross the TCP wire in the request.
            envelope.auth_token = Some(String::from_utf8_lossy(secret).into_owned());
            match select_tab(&envelope, &state) {
                Ok(selected) => (handle(envelope, &selected), selected),
                Err(message) => (
                    vec![error(
                        &envelope.request_id,
                        if message.starts_with("session_unauthorized:") {
                            "session_unauthorized"
                        } else {
                            "tab_open_failed"
                        },
                        message,
                    )],
                    default_tab.clone(),
                ),
            }
        }
        Ok(_) => (
            vec![error(
                &request_id,
                "authentication_failed",
                "request id does not match the authenticated challenge",
            )],
            default_tab,
        ),
        Err(error_value) => (
            vec![error("unknown", "invalid_request", error_value.to_string())],
            default_tab,
        ),
    };
    write_frames(reader.get_mut(), value, &selected_tab, &state);
}

fn stream_read_frames(request_id: &str, tab: &Tab) -> Vec<Value> {
    let Ok(text) = tab.document.text() else {
        return vec![error(
            request_id,
            "invalid_utf8",
            "streaming text reads require a UTF-8 document",
        )];
    };
    let mut frames = Vec::new();
    let mut start = 0;
    for (offset, _) in text.char_indices().skip(1) {
        if offset - start >= 64 * 1024 {
            frames.push(response(
                request_id,
                json!({"text": &text[start..offset], "revision": tab.revision, "stream": true, "complete": false}),
            ));
            start = offset;
        }
    }
    frames.push(response(
        request_id,
        json!({"text": &text[start..], "revision": tab.revision, "stream": true, "complete": true}),
    ));
    frames
}

fn write_frames<S: std::io::Write>(
    writer: &mut S,
    frames: Vec<Value>,
    tab: &Arc<Mutex<Tab>>,
    state: &Arc<Mutex<ServerState>>,
) {
    let stream_revision = frames.iter().find_map(|frame| {
        (frame.get("type").and_then(Value::as_str) == Some("data")
            && frame.pointer("/payload/stream").and_then(Value::as_bool) == Some(true))
        .then(|| frame.pointer("/payload/revision").and_then(Value::as_u64))
        .flatten()
    });
    for (sequence, mut frame) in frames.into_iter().enumerate() {
        if let Some(expected_revision) = stream_revision {
            let current_revision = tab.lock().ok().map(|tab| tab.revision);
            if current_revision != Some(expected_revision) {
                let Some(revision) = current_revision else {
                    let request_id = frame
                        .get("request_id")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown");
                    let bytes = serde_json::to_vec(&error(
                        request_id,
                        "stream_conflict",
                        "cannot verify the current revision because the tab lock is poisoned",
                    ))
                    .unwrap();
                    if writer
                        .write_all(&[bytes.as_slice(), b"\n"].concat())
                        .is_err()
                    {
                        return;
                    }
                    return;
                };
                let delimiter = response(
                    frame
                        .get("request_id")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown"),
                    json!({"text": "===== FILE EDITED: RESTARTING =====\n", "revision": revision, "stream": true, "restart": true, "complete": false}),
                );
                let mut replacement = tab
                    .lock()
                    .ok()
                    .map(|tab| {
                        stream_read_frames(
                            frame
                                .get("request_id")
                                .and_then(Value::as_str)
                                .unwrap_or("unknown"),
                            &tab,
                        )
                    })
                    .unwrap_or_default();
                replacement.push(complete(
                    frame
                        .get("request_id")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown"),
                    &revision.to_string(),
                ));
                let mut restart_frames = Vec::with_capacity(replacement.len() + 1);
                restart_frames.push(delimiter);
                restart_frames.extend(replacement);
                for (restart_sequence, mut restart_frame) in restart_frames.into_iter().enumerate()
                {
                    if let Some(object) = restart_frame.as_object_mut() {
                        object.insert(
                            "sequence".into(),
                            json!((sequence + restart_sequence) as u64),
                        );
                    }
                    let mut restart_bytes = serde_json::to_vec(&restart_frame).unwrap();
                    restart_bytes.push(b'\n');
                    if writer.write_all(&restart_bytes).is_err() {
                        return;
                    }
                }
                break;
            }
        }
        if let Some(object) = frame.as_object_mut() {
            object.insert("sequence".into(), json!(sequence as u64));
            if object.get("type").and_then(Value::as_str) == Some("data") {
                let byte_count = object
                    .get("payload")
                    .map(ai_text_editor::protocol::canonical_json)
                    .map(|payload| payload.len())
                    .unwrap_or(0);
                object.insert("byte_count".into(), json!(byte_count));
            }
        }
        let mut bytes = serde_json::to_vec(&frame).unwrap();
        if bytes.len() > MAX_SERIALIZED_FRAME_BYTES {
            let request_id = frame
                .get("request_id")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_owned();
            frame = error(
                &request_id,
                "response_too_large",
                "response exceeds the 8 MiB protocol frame limit; reduce the range, limit, or page size",
            );
            if let Some(object) = frame.as_object_mut() {
                object.insert("sequence".into(), json!(sequence as u64));
            }
            bytes = serde_json::to_vec(&frame).unwrap();
        }
        bytes.push(b'\n');
        if writer.write_all(&bytes).is_err() {
            return;
        }
    }
    let _ = writer.flush();
    let closed_path = tab
        .lock()
        .ok()
        .and_then(|tab| tab.close_after_response.then(|| tab.path.clone()));
    if let Some(path) = closed_path {
        let key = tab_key(&path);
        let mut state_guard = state.lock().unwrap();
        state_guard.tabs.remove(&key);
        if state_guard.tabs.is_empty() {
            let discovery_path = state_guard.discovery_path.clone();
            let _ = fs::remove_file(endpoint_for_file(&discovery_path));
            let _ = fs::remove_file(socket_for_file(&discovery_path));
            std::process::exit(0);
        }
        if state_guard.default_key == key {
            if let Some(next) = state_guard.tabs.keys().next().cloned() {
                state_guard.default_key = next;
            }
        }
    }
}

fn server_generation() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    blake3::hash(
        format!(
            "{}:{}:{}",
            std::process::id(),
            now.as_secs(),
            now.subsec_nanos()
        )
        .as_bytes(),
    )
    .to_hex()
    .to_string()
}

fn tab_key(path: &std::path::Path) -> String {
    let identity =
        ai_text_editor::transport::canonical_or_near(path).unwrap_or_else(|_| path.to_path_buf());
    blake3::hash(identity.to_string_lossy().as_bytes())
        .to_hex()
        .to_string()
}

fn default_tab(state: &Arc<Mutex<ServerState>>) -> Arc<Mutex<Tab>> {
    let state = state.lock().unwrap();
    state.tabs[&state.default_key].clone()
}

/// The file a request explicitly names, in the same canonical form tab
/// paths are stored in, or `None` when the request names none.
fn requested_file(envelope: &ai_text_editor::protocol::Envelope) -> Option<PathBuf> {
    envelope
        .payload
        .get("file")
        .and_then(Value::as_str)
        .map(|path| {
            ai_text_editor::transport::canonical_or_near(std::path::Path::new(path))
                .unwrap_or_else(|_| std::path::PathBuf::from(path))
        })
}

/// A tab may only serve a request that names its own file. Routing was
/// token-first with `file` consulted only for `open`, which let a
/// `-f OTHERFILE` replace silently mutate the session's *previous* tab:
/// the edit returned a real new revision, the next read (routed elsewhere)
/// never showed it, and the named file never changed on disk.
fn ensure_tab_file(
    tab: &Arc<Mutex<Tab>>,
    requested: Option<&PathBuf>,
    method: &str,
) -> Result<(), String> {
    // `capabilities` and `resources` describe the server, not the tab's
    // content, so a file named with them is only a discovery hint and must
    // not be refused for addressing a different tab.
    if matches!(method, "capabilities" | "resources") {
        return Ok(());
    }
    let Some(requested) = requested else {
        return Ok(());
    };
    let held = tab.lock().ok().map(|tab| tab.path.clone());
    match held {
        Some(path) if &path == requested => Ok(()),
        Some(path) => Err(format!(
            "file_mismatch: the request names {requested:?} but this tab holds {}; run `ai-text-editor open -f {}` to route to the named file",
            path.display(),
            requested.display()
        )),
        None => Err("tab_unavailable: the tab lock is poisoned".into()),
    }
}

fn select_tab(
    envelope: &ai_text_editor::protocol::Envelope,
    state: &Arc<Mutex<ServerState>>,
) -> Result<Arc<Mutex<Tab>>, String> {
    let state_guard = state.lock().unwrap();
    let requested = requested_file(envelope);
    if let Some(token) = envelope.session_token.as_deref() {
        for tab in state_guard.tabs.values() {
            if tab
                .lock()
                .ok()
                .is_some_and(|tab| tab.session_token == token)
            {
                ensure_tab_file(tab, requested.as_ref(), &envelope.method)?;
                return Ok(tab.clone());
            }
        }
        return Err(
            "session_unauthorized: the supplied session_token does not belong to a tab on this server"
                .into(),
        );
    }
    if envelope.method == "open" {
        if let Some(path) = requested.as_ref() {
            let key = tab_key(path);
            if let Some(tab) = state_guard.tabs.get(&key) {
                return Ok(tab.clone());
            }
            let mode = state_guard.mode;
            let normalize_nfc = state_guard.normalize_nfc;
            let auth_token = state_guard.auth_token.clone();
            let generation = state_guard.server_generation.clone();
            let threshold = state_guard.large_threshold_bytes;
            let endpoint = state_guard.endpoint.clone();
            drop(state_guard);
            let tab = open_additional_tab(
                path.clone(),
                mode,
                normalize_nfc,
                auth_token.clone(),
                generation.clone(),
                threshold,
            )?;
            if let Some(endpoint) = endpoint {
                let tab_guard = tab;
                let session_token = tab_guard.session_token.clone();
                register_session(
                    &endpoint,
                    &generation,
                    &session_token,
                    auth_token.as_deref(),
                );
                let tab = Arc::new(Mutex::new(tab_guard));
                let key = tab_key(path);
                let mut state_guard = state.lock().unwrap();
                if let Some(existing) = state_guard.tabs.get(&key) {
                    return Ok(existing.clone());
                }
                let result = tab.clone();
                state_guard.tabs.insert(key, tab);
                return Ok(result);
            }
            let key = tab_key(path);
            let tab = Arc::new(Mutex::new(tab));
            let mut state_guard = state.lock().unwrap();
            if let Some(existing) = state_guard.tabs.get(&key) {
                return Ok(existing.clone());
            }
            let result = tab.clone();
            state_guard.tabs.insert(key, tab);
            return Ok(result);
        }
    }
    let default = state_guard.tabs[&state_guard.default_key].clone();
    ensure_tab_file(&default, requested.as_ref(), &envelope.method)?;
    Ok(default)
}

fn handle(envelope: ai_text_editor::protocol::Envelope, tab: &Arc<Mutex<Tab>>) -> Vec<Value> {
    let mut tab = tab.lock().unwrap();
    let mut frames = Vec::new();
    if let Some(expected) = &tab.auth_token {
        if envelope.auth_token.as_deref() != Some(expected.as_str()) {
            frames.push(error(
                &envelope.request_id,
                "authentication_failed",
                "auth_token is required or invalid",
            ));
            return frames;
        }
    }
    if envelope.method != "open"
        && envelope.session_token.as_deref() != Some(tab.session_token.as_str())
    {
        frames.push(error(
            &envelope.request_id,
            "session_unauthorized",
            "a valid server-issued session_token is required for this tab operation; if the server restarted, run `open` again (the journal replays)",
        ));
        return frames;
    }
    // B180/B187: refuse arguments no handler for this verb reads. A typo'd
    // or misplaced flag must fail here, not become a silent no-op.
    if let Some(key) = envelope
        .payload
        .as_object()
        .and_then(|map| {
            map.keys()
                .find(|key| !KNOWN_PAYLOAD_KEYS.contains(&key.as_str()))
        })
        .cloned()
    {
        frames.push(error_details(
            &envelope.request_id,
            "unknown_argument",
            format!(
                "the request carries {key}, which no handler for {} reads",
                envelope.method
            ),
            json!({"offending_key": key}),
        ));
        return frames;
    }
    if envelope.method == "search" && envelope.payload.get("offset").is_some() {
        frames.push(error(
            &envelope.request_id,
            "search_offset_unsupported",
            "search always scans the whole (bounded) document; page an existing result set with `page --pager-key <key> --offset N` instead",
        ));
        return frames;
    }
    if envelope.method == "read" && envelope.payload.get("historical").is_some() {
        frames.push(error(
            &envelope.request_id,
            "read_historical_unsupported",
            "historical belongs to search and page; a read always returns the live buffer",
        ));
        return frames;
    }
    if envelope.method != "resolve_external" && envelope.method != "save_as" {
        observe_external(&mut tab);
    }
    // An unresolved external change blocks everything that could lose a
    // side's bytes — but not the read-only commands. A read that answers
    // "resolve first" with an error frame is how a wedged session looks
    // under `-p text`: an empty, silent command the agent cannot diagnose;
    // the buffer is what the editor holds, and that is exactly what a
    // confused agent needs to see while deciding.
    if let Some(bytes) = &tab.pending_external {
        if matches!(
            envelope.method.as_str(),
            "insert"
                | "replace"
                | "undo"
                | "redo"
                | "save"
                | "large_edit"
                | "restore"
                | "begin_transaction"
                | "end_transaction"
                | "job_start"
                | "job_complete"
        ) {
            let byte_count = bytes.len();
            frames.push(error_details(&envelope.request_id, "external_change", "the file changed outside this editor tab; mutating commands are blocked until you resolve the external change (reads still return the editor's buffer)", json!({"bytes": byte_count, "choices": ["backup", "reload", "merge", "keep", "force_save"], "force_save_requires": "acknowledge_force_save=true"})));
            return frames;
        }
    }
    if matches!(
        envelope.method.as_str(),
        "insert" | "replace" | "undo" | "redo" | "save" | "large_edit" | "restore"
    ) {
        if tab.large_file.is_some()
            && !matches!(envelope.method.as_str(), "large_edit" | "undo" | "redo")
        {
            frames.push(error(&envelope.request_id, "large_file_edit_requires_acknowledgement", "large tabs provide bounded access; editing requires an explicit long-operation acknowledgement"));
            return frames;
        }
        let Some(expected) = envelope.revision else {
            if envelope.method == "large_edit" {
                fail_guarded_job(
                    &mut tab,
                    &envelope,
                    "large_edit refused before the rewrite: revision_required",
                );
            }
            frames.push(error(
                &envelope.request_id,
                "revision_required",
                "mutating operations require the current revision; read open or history first",
            ));
            return frames;
        };
        if let Err(revision_error) = RevisionGuard(expected).check(tab.revision) {
            if envelope.method == "large_edit" {
                fail_guarded_job(
                    &mut tab,
                    &envelope,
                    "large_edit refused before the rewrite: stale revision",
                );
            }
            frames.push(error(
                &envelope.request_id,
                "stale_revision",
                revision_error.to_string(),
            ));
            return frames;
        }
    }
    match envelope.method.as_str() {
        "open" => {
            let mut payload = json!({"path": tab.path, "mode": tab.document.mode, "normalize_nfc": tab.document.normalize_nfc, "revision": tab.revision, "dirty": tab_dirty(&tab), "disk_diverged": tab_disk_diverged(&tab), "external_change_pending": tab.pending_external.is_some(), "bytes": tab.large_file.as_ref().map(|file| file.bytes).unwrap_or(tab.document.bytes().len() as u64), "large_file": tab.large_file.is_some(), "index_loaded": tab.index_loaded, "index_complete": tab.index_complete, "index_coverage": {"through_line": tab.index.blocks.last().map(|block| block.line).unwrap_or(0), "through_byte": tab.index.blocks.last().map(|block| block.byte_offset).unwrap_or(0)}, "cursors": tab.cursors, "session_token": tab.session_token, "tab_uuid": session::tab_uuid_for(&tab.session_token, &tab.server_generation), "server_generation": tab.server_generation, "server_pid": std::process::id(), "resources": resources::report(tab.document.bytes().len(), tab.large_threshold_bytes)});
            if tab.replayed_edits > 0 {
                payload["journal_replay"] =
                    json!({"edits": tab.replayed_edits, "through_sequence": tab.journal_seq});
            }
            frames.push(response(&envelope.request_id, payload));
        }
        "capabilities" => frames.push(response(&envelope.request_id, json!({
            "protocol_version": ai_text_editor::PROTOCOL_VERSION,
            "document_modes": ["text_utf8", "raw_bytes", "hex_view"],
            "search_modes": ["exact_text", "exact_bytes", "wildcard", "shell_wildcard", "path_wildcard", "regex_rust", "regex_pcre2", "fuzzy_edit", "fuzzy_subsequence", "fuzzy_token", "fuzzy_ngram", "fuzzy_phonetic", "fuzzy_soundex"],
            "presentations": ["structured", "text", "paging", "stream"],
            "defaults": {"presentation": "structured", "search_preview_matches": 4, "index_granularity": DEFAULT_GRANULARITY, "large_file_threshold_bytes": tab.large_threshold_bytes},
            "coordinates": {"text": {"line_base": 1, "column_base": 0, "column_unit": "unicode_scalar"}, "raw_bytes": {"line_base": 1, "column_base": 0, "column_unit": "byte"}, "hex_view": {"row_bytes": 16, "column_base": 0, "column_unit": "byte"}},
            "fuzzy_gradient": {"range": [0.0, 1.0], "edit": "permitted_distance_fraction", "subsequence_token_ngram": "minimum_score", "phonetic_soundex": "binary_match_score"},
            "large_file": {"bounded_reads": true, "ordinary_mutations": false, "acknowledged_job_edits": true},
            "revision_required_methods": ["insert", "replace", "large_edit", "restore", "undo", "redo", "save"],
            "transports": ["unix_socket", "loopback_tcp"]
        }))),
        "resources" => frames.push(response(&envelope.request_id, json!(resources::report(tab.document.bytes().len(), tab.large_threshold_bytes)))),
        "history" => {
            let (undo_depth, redo_depth) = tab.history.depths();
            frames.push(response(&envelope.request_id, json!({"revision": tab.revision, "undo_depth": undo_depth + tab.large_undo.len(), "redo_depth": redo_depth + tab.large_redo.len(), "text_undo_depth": undo_depth, "text_redo_depth": redo_depth, "large_undo_depth": tab.large_undo.len(), "large_redo_depth": tab.large_redo.len(), "journal_sequence": tab.journal_seq})));
        }
        "begin_transaction" => {
            if tab.transaction_before.is_some() {
                frames.push(error(&envelope.request_id, "transaction_already_open", "an undo transaction is already open"));
            } else {
                tab.transaction_before = Some(tab.document.clone());
                frames.push(response(&envelope.request_id, json!({"transaction": "open", "revision": tab.revision})));
            }
        }
        "end_transaction" => {
            let Some(before) = tab.transaction_before.take() else {
                frames.push(error(&envelope.request_id, "no_transaction", "no undo transaction is open"));
                return frames;
            };
            if before.bytes() == tab.document.bytes() {
                frames.push(response(&envelope.request_id, json!({"transaction": "closed", "changed": false, "revision": tab.revision})));
            } else {
                let after = tab.document.clone();
                tab.history.record(&before, &after);
                let revision = tab.revision;
                journal_append(&mut tab, "transaction_end", json!({"revision": revision}));
                frames.push(response(&envelope.request_id, json!({"transaction": "closed", "changed": true, "revision": tab.revision})));
            }
        }
        "restore" => restore_document(&envelope, &mut tab, &mut frames),
        "index" => {
            let granularity = envelope.payload.get("granularity").and_then(Value::as_u64).unwrap_or(DEFAULT_GRANULARITY as u64) as usize;
            if let Some(file) = &tab.large_file {
                match file.index(granularity) {
                    Ok(index) => tab.index = index,
                    Err(error_value) => { frames.push(error(&envelope.request_id, "index_failed", error_value.to_string())); return frames; }
                }
            } else {
                tab.index = LineIndex::build(tab.document.bytes(), granularity);
            }
            tab.index_complete = true;
            persist_index(&mut tab);
            let block_offset = envelope
                .payload
                .get("offset")
                .and_then(Value::as_u64)
                .unwrap_or(0) as usize;
            let block_limit = envelope
                .payload
                .get("limit")
                .and_then(Value::as_u64)
                .unwrap_or(4) as usize;
            let blocks = tab
                .index
                .blocks
                .get(block_offset..)
                .unwrap_or(&[])
                .iter()
                .take(block_limit)
                .cloned()
                .collect::<Vec<_>>();
            let returned_blocks = blocks.len();
            frames.push(response(&envelope.request_id, json!({"granularity": tab.index.granularity, "bytes": tab.index.bytes, "lines": tab.index.lines, "blocks": blocks, "block_offset": block_offset, "block_count": tab.index.blocks.len(), "returned_blocks": returned_blocks, "complete": tab.index_complete, "coverage": {"through_line": tab.index.blocks.last().map(|block| block.line).unwrap_or(0), "through_byte": tab.index.blocks.last().map(|block| block.byte_offset).unwrap_or(0)}})));
        }
        "cursor" => cursor(&envelope, &mut tab, &mut frames),
        "page" => page(&envelope, &mut tab, &mut frames),
        "read" => {
            // The documented read bounds (the client help's "Reading:" line):
            // range_start_line/range_end_line are one-based and INCLUSIVE on
            // text tabs; range_start_byte/range_end_byte are zero-based and
            // HALF-OPEN on raw and hex tabs. A bound the tab cannot honor is
            // refused with the flag named — silently ignoring one is B171.
            let line_range = match (
                envelope.payload.get("range_start_line").and_then(Value::as_u64),
                envelope.payload.get("range_end_line").and_then(Value::as_u64),
            ) {
                (None, None) => None,
                (Some(start), Some(end)) => {
                    if start == 0 || end < start {
                        frames.push(error(&envelope.request_id, "read_range_invalid", "range_start_line must be positive and range_end_line must not precede it"));
                        return frames;
                    }
                    Some((start, end))
                }
                (Some(start), None) => {
                    frames.push(error_details(
                        &envelope.request_id,
                        "read_range_incomplete",
                        "a line-range read requires an explicit inclusive range; range_end_line was omitted",
                        json!({"range_start_line": start, "choices": ["provide range_end_line", "omit range_start_line to read the whole document"]}),
                    ));
                    return frames;
                }
                (None, Some(_)) => {
                    frames.push(error(&envelope.request_id, "read_range_incomplete", "range_end_line was given without range_start_line"));
                    return frames;
                }
            };
            let byte_range = match (
                envelope.payload.get("range_start_byte").and_then(Value::as_u64),
                envelope.payload.get("range_end_byte").and_then(Value::as_u64),
            ) {
                (None, None) => None,
                (Some(start), Some(end)) => {
                    if end < start {
                        frames.push(error(&envelope.request_id, "read_range_invalid", "range_end_byte must not precede range_start_byte"));
                        return frames;
                    }
                    Some((start, end))
                }
                _ => {
                    frames.push(error(&envelope.request_id, "read_range_incomplete", "byte-range reads require both range_start_byte and range_end_byte"));
                    return frames;
                }
            };
            if line_range.is_some() && byte_range.is_some() {
                frames.push(error(&envelope.request_id, "read_range_conflict", "a read takes either a line range or a byte range, not both"));
                return frames;
            }
            let windowed = envelope.payload.get("offset").is_some()
                || envelope.payload.get("before").is_some()
                || envelope.payload.get("after").is_some()
                || envelope.payload.get("line").is_some();
            if windowed && (line_range.is_some() || byte_range.is_some()) {
                frames.push(error(&envelope.request_id, "read_range_conflict", "a bounded read takes either offset/length/before/after/line or a range bound, not both"));
                return frames;
            }
            if (line_range.is_some() || byte_range.is_some())
                && envelope.payload.get("presentation").and_then(Value::as_str) == Some("stream")
            {
                frames.push(error(&envelope.request_id, "read_range_conflict", "streamed reads deliver the whole document; drop the range or the stream presentation"));
                return frames;
            }
            if let Some(file) = &tab.large_file {
                if let Some((start_line, end_line)) = line_range {
                    if tab.document.mode != DocumentMode::TextUtf8 {
                        frames.push(error(&envelope.request_id, "read_range_unsupported", "line-range reads apply to text tabs; a large raw or hex tab takes range_start_byte/range_end_byte"));
                        return frames;
                    }
                    let block = tab.index.block_for_line(start_line as usize).clone();
                    match file.read_lines_from(start_line, (end_line - start_line + 1) as usize, block.byte_offset as u64, block.line) {
                        Ok(lines) => frames.push(response(&envelope.request_id, json!({"text": lines.text, "start_line": lines.start_line, "end_line": lines.end_line, "eof": lines.eof, "revision": tab.revision, "large_file": true}))),
                        Err(error_value) => frames.push(error(&envelope.request_id, "large_read_failed", error_value.to_string())),
                    }
                    return frames;
                }
                if let Some((start_byte, end_byte)) = byte_range {
                    match file.read_range(start_byte, (end_byte - start_byte) as usize) {
                        Ok(range) => {
                            let decoded = if tab.document.mode == DocumentMode::TextUtf8 {
                                std::str::from_utf8(&range.bytes).ok().map(str::to_owned)
                            } else {
                                None
                            };
                            let mut payload = json!({"offset": range.offset, "bytes_base64": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, range.bytes), "eof": range.eof, "revision": tab.revision, "dirty": tab_dirty(&tab), "disk_diverged": tab_disk_diverged(&tab), "large_file": true});
                            if let Some(text) = decoded {
                                payload["text"] = json!(text);
                            }
                            frames.push(response(&envelope.request_id, payload));
                        }
                        Err(error_value) => frames.push(error(&envelope.request_id, "large_read_failed", error_value.to_string())),
                    }
                    return frames;
                }
                if envelope.payload.get("line").is_some()
                    || envelope.payload.get("before").is_some()
                    || envelope.payload.get("after").is_some()
                {
                    let line = envelope
                        .payload
                        .get("line")
                        .and_then(Value::as_u64)
                        .unwrap_or(1)
                        .max(1);
                    let before = envelope
                        .payload
                        .get("before")
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    let after = envelope
                        .payload
                        .get("after")
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    let start_line = line.saturating_sub(before).max(1);
                    let line_count = before.saturating_add(after).saturating_add(1) as usize;
                    let block = tab.index.block_for_line(start_line as usize).clone();
                    match file.read_lines_from(
                        start_line,
                        line_count,
                        block.byte_offset as u64,
                        block.line,
                    ) {
                        Ok(lines) => frames.push(response(&envelope.request_id, json!({"text": lines.text, "start_line": lines.start_line, "end_line": lines.end_line, "eof": lines.eof, "revision": tab.revision, "large_file": true}))),
                        Err(error_value) => frames.push(error(&envelope.request_id, "large_read_failed", error_value.to_string())),
                    }
                    return frames;
                }
                let offset = envelope.payload.get("offset").and_then(Value::as_u64).unwrap_or(0);
                let requested = envelope.payload.get("length").and_then(Value::as_u64).unwrap_or(ai_text_editor::large_file::DEFAULT_READ_BYTES as u64) as usize;
                match file.read_range(offset, requested) {
                    Ok(range) => {
                        let decoded = if tab.document.mode == DocumentMode::TextUtf8 {
                            std::str::from_utf8(&range.bytes).ok().map(str::to_owned)
                        } else {
                            None
                        };
                        let mut payload = json!({"offset": range.offset, "bytes_base64": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, range.bytes), "eof": range.eof, "revision": tab.revision, "large_file": true});
                        if let Some(text) = decoded {
                            payload["text"] = json!(text);
                        }
                        frames.push(response(&envelope.request_id, payload));
                    }
                    Err(error_value) => frames.push(error(&envelope.request_id, "large_read_failed", error_value.to_string())),
                }
                return frames;
            }
            if tab.document.mode == DocumentMode::TextUtf8 {
                match tab.document.text() {
                    Ok(text) => {
                        let before = envelope.payload.get("before").and_then(Value::as_u64);
                        let after = envelope.payload.get("after").and_then(Value::as_u64);
                        if let Some((start_line, end_line)) = line_range {
                            let lines: Vec<&str> = text.split_inclusive('\n').collect();
                            let start = (start_line as usize).min(lines.len() + 1);
                            let end = (end_line as usize).min(lines.len());
                            let selected = if start <= end { lines[start - 1..end].concat() } else { String::new() };
                            frames.push(response(&envelope.request_id, json!({"text": selected, "revision": tab.revision, "start_line": start, "end_line": end, "dirty": tab_dirty(&tab), "disk_diverged": tab_disk_diverged(&tab), "complete": start == 1 && end == lines.len()})));
                        } else if byte_range.is_some() {
                            frames.push(error(&envelope.request_id, "read_range_unsupported", "a text tab reads a byte window with offset/length, snapped to char boundaries; range_start_byte/range_end_byte address raw and hex tabs"));
                        } else if before.is_some() || after.is_some() {
                            let cursor_id = envelope
                                .payload
                                .get("cursor_id")
                                .and_then(Value::as_u64)
                                .unwrap_or(0);
                            let cursor = tab
                                .cursors
                                .get(&cursor_id)
                                .copied()
                                .unwrap_or(Position { line: 1, column: 0 });
                            let lines: Vec<&str> = text.split_inclusive('\n').collect();
                            let start = cursor.line.saturating_sub(before.unwrap_or(0) as usize + 1);
                            let end = (cursor.line.saturating_add(after.unwrap_or(0) as usize)).min(lines.len());
                            let selected = lines.get(start..end).unwrap_or(&[]).concat();
                            frames.push(response(&envelope.request_id, json!({"text": selected, "revision": tab.revision, "start_line": start + 1, "end_line": end, "cursor": cursor, "dirty": tab_dirty(&tab), "disk_diverged": tab_disk_diverged(&tab), "complete": end == lines.len() && start == 0})))
                        } else if let Some(offset) = envelope.payload.get("offset").and_then(Value::as_u64) {
                            // -o/--offset and -L/--length address a text read
                            // in BYTES (matching raw/hex coordinates), and
                            // the window is snapped outward from an offset
                            // and inward from a length so both ends land on
                            // UTF-8 char boundaries.
                            let mut start = (offset as usize).min(text.len());
                            while start < text.len() && !text.is_char_boundary(start) {
                                start += 1;
                            }
                            let mut end = match envelope.payload.get("length").and_then(Value::as_u64) {
                                Some(length) => start.saturating_add(length as usize).min(text.len()),
                                None => text.len(),
                            };
                            while end > start && !text.is_char_boundary(end) {
                                end -= 1;
                            }
                            frames.push(response(&envelope.request_id, json!({"text": &text[start..end], "revision": tab.revision, "offset": start, "returned_bytes": end - start, "total_bytes": text.len(), "eof": end == text.len(), "dirty": tab_dirty(&tab), "disk_diverged": tab_disk_diverged(&tab), "complete": end == text.len()})));
                        } else {
                            if envelope.payload.get("presentation").and_then(Value::as_str) == Some("stream") {
                                frames.extend(stream_read_frames(&envelope.request_id, &tab));
                            } else {
                                frames.push(response(&envelope.request_id, json!({"text": text, "revision": tab.revision, "dirty": tab_dirty(&tab), "disk_diverged": tab_disk_diverged(&tab), "complete": true})))
                            }
                        }
                    }
                    Err(error_value) => frames.push(error(&envelope.request_id, "invalid_utf8", error_value.to_string())),
                }
            } else if let Some((start_byte, end_byte)) = byte_range {
                let bytes = tab.document.bytes();
                let start = (start_byte as usize).min(bytes.len());
                let end = (end_byte as usize).min(bytes.len()).max(start);
                frames.push(response(&envelope.request_id, json!({"bytes_base64": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes[start..end]), "offset": start, "returned_bytes": end - start, "total_bytes": bytes.len(), "eof": end == bytes.len(), "revision": tab.revision, "dirty": tab_dirty(&tab), "disk_diverged": tab_disk_diverged(&tab), "mode": tab.document.mode})));
            } else if line_range.is_some() {
                frames.push(error(&envelope.request_id, "read_range_unsupported", "line-range reads apply to text tabs; raw and hex tabs take range_start_byte/range_end_byte (half-open)"));
            } else {
                frames.push(response(&envelope.request_id, json!({"bytes_base64": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, tab.document.bytes()), "revision": tab.revision, "dirty": tab_dirty(&tab), "disk_diverged": tab_disk_diverged(&tab), "mode": tab.document.mode})))
            }
        },
        "replace" | "insert" => {
            let offset = envelope
                .payload
                .get("offset")
                .and_then(Value::as_u64)
                .map(|value| value as usize)
                .unwrap_or_else(|| {
                    let cursor_id = envelope
                        .payload
                        .get("cursor_id")
                        .and_then(Value::as_u64)
                        .unwrap_or(0);
                    position_offset(
                        &tab.document,
                        tab.cursors
                            .get(&cursor_id)
                            .copied()
                            .unwrap_or(Position { line: 1, column: 0 }),
                    )
                });
            let delete_len = if envelope.method == "insert" { 0 } else { envelope.payload.get("delete_len").and_then(Value::as_u64).unwrap_or(0) as usize };
            let replacement = if let Some(encoded) = envelope.payload.get("bytes_base64").and_then(Value::as_str) {
                match base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded) { Ok(bytes) => bytes, Err(error_value) => { frames.push(error(&envelope.request_id, "invalid_base64", error_value.to_string())); return frames; } }
            } else { envelope.payload.get("text").and_then(Value::as_str).unwrap_or("").as_bytes().to_vec() };
            let before = tab.document.clone();
            let mut after = before.clone();
            match after.apply_bytes(offset, delete_len, &replacement) {
                Ok(()) => {
                    let revision = tab.revision.saturating_add(1);
                    let record = json!({"offset": offset, "delete_len": delete_len, "bytes": replacement.len(), "revision": revision, "before": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, before.bytes()), "after": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, after.bytes())});
                    match journal_append_result(&mut tab, "edit", record) {
                        Ok(()) => {
                            adjust_cursors(
                                &before,
                                &after,
                                &mut tab.cursors,
                                offset,
                                delete_len,
                                replacement.len(),
                            );
                            tab.document = after.clone();
                            if tab.transaction_before.is_none() {
                                tab.history.record(&before, &after);
                            }
                            tab.revision = revision;
                            tab.results.clear();
                            tab.index = LineIndex::build(tab.document.bytes(), tab.index.granularity);
                            tab.index_complete = true;
                            persist_index(&mut tab);
                            let _ = tab.metadata.record(&tab.path, tab.document.mode, tab.revision, tab.document.bytes().len());
                            // A delete spanning a line end joins two lines;
                            // nothing in the coordinates says so otherwise.
                            let spans_lines =
                                before.bytes()[offset..offset + delete_len].contains(&b'\n');
                            let mut payload = json!({"revision": tab.revision, "cursors": tab.cursors, "dirty": tab_dirty(&tab), "disk_diverged": tab_disk_diverged(&tab)});
                            if spans_lines {
                                payload["spans_lines"] = json!(true);
                            }
                            frames.push(response(&envelope.request_id, payload));
                        }
                        Err(error_value) => frames.push(error(&envelope.request_id, "journal_write_failed", error_value.to_string())),
                    }
                }
                Err(error_value) => {
                    let message = if tab.document.normalize_nfc
                        && tab.document.mode == DocumentMode::TextUtf8
                    {
                        format!(
                            "{error_value}; on a normalized tab, byte offsets address the ORIGINAL bytes, not the normalized view a read shows - offset {offset} is inside a multi-byte sequence of the original"
                        )
                    } else {
                        error_value.to_string()
                    };
                    frames.push(error(&envelope.request_id, "edit_refused", message));
                }
            }
        }
        "large_edit" => large_edit(&envelope, &mut tab, &mut frames),
        "undo" => history_step(&envelope, &mut tab, &mut frames, false),
        "redo" => history_step(&envelope, &mut tab, &mut frames, true),
        "save" => save(&envelope, &mut tab, &mut frames),
        "save_as" => save_as(&envelope, &mut tab, &mut frames),
        "close" => close_tab(&envelope, &mut tab, &mut frames),
        "resolve_external" => resolve_external(&envelope, &mut tab, &mut frames),
        "search" => search(&envelope, &mut tab, &mut frames),
        "job_start" => job_start(&envelope, &mut tab, &mut frames),
        "job_poll" => job_poll(&envelope, &mut tab, &mut frames),
        "job_progress" => job_progress(&envelope, &mut tab, &mut frames),
        "job_complete" => job_complete(&envelope, &mut tab, &mut frames),
        "job_cancel" => job_cancel(&envelope, &mut tab, &mut frames),
        "job_transfer" => job_transfer(&envelope, &mut tab, &mut frames),
        "job_release" => job_release(&envelope, &mut tab, &mut frames),
        _ => frames.push(error(&envelope.request_id, "unknown_method", format!("unsupported method {}", envelope.method))),
    }
    if !frames
        .iter()
        .any(|frame| frame.get("type").and_then(Value::as_str) == Some("error"))
    {
        frames.push(complete(&envelope.request_id, &tab.revision.to_string()));
    }
    frames
}

fn restore_document(
    envelope: &ai_text_editor::protocol::Envelope,
    tab: &mut Tab,
    frames: &mut Vec<Value>,
) {
    let before = tab.document.clone();
    let mut after = before.clone();
    if let Err(error_value) = after.restore_original() {
        frames.push(error(
            &envelope.request_id,
            "restoration_conflict",
            error_value.to_string(),
        ));
        return;
    }
    if !before.normalize_nfc {
        frames.push(error(
            &envelope.request_id,
            "not_normalized",
            "this tab was not opened with NFC normalization; there is no normalized presentation to restore away from",
        ));
        return;
    }
    let revision = tab.revision.saturating_add(1);
    if let Err(error_value) = journal_append_result(
        tab,
        "restore",
        json!({
            "revision": revision,
            "before": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, before.bytes()),
            "after": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, after.bytes())
        }),
    ) {
        frames.push(error(
            &envelope.request_id,
            "journal_write_failed",
            error_value.to_string(),
        ));
        return;
    }
    tab.document = after.clone();
    tab.history.record(&before, &after);
    tab.revision = revision;
    tab.results.clear();
    frames.push(response(
        &envelope.request_id,
        json!({"restored": true, "normalize_nfc": false, "revision": revision}),
    ));
}

fn observe_external(tab: &mut Tab) {
    if tab.pending_external.is_some() {
        return;
    }
    if let Some(large) = &tab.large_file {
        if disk_state(&tab.path, Some(large), &[]) != tab.disk_digest {
            tab.pending_external = Some(Vec::new());
        }
    } else if let Ok(bytes) = fs::read(&tab.path) {
        if disk_state(&tab.path, None, &bytes) != tab.disk_digest {
            tab.pending_external = Some(bytes);
        }
    }
}

fn tab_base_bytes(path: &std::path::Path) -> Vec<u8> {
    fs::read(path).unwrap_or_default()
}

fn journal_append_result(tab: &mut Tab, kind: &str, payload: Value) -> io::Result<()> {
    tab.journal_seq = tab.journal_seq.saturating_add(1);
    let record = JournalRecord {
        tx_id: format!("tab-{}", tab.revision),
        seq: tab.journal_seq,
        kind: kind.to_owned(),
        payload,
    };
    tab.journal.append(&record).map(|_| ())
}

fn journal_append(tab: &mut Tab, kind: &str, payload: Value) {
    if let Err(error_value) = journal_append_result(tab, kind, payload) {
        eprintln!("ai-text-editor-server: journal append failed: {error_value}");
    }
}

fn digest(bytes: &[u8]) -> String {
    blake3::hash(bytes).to_hex().to_string()
}

fn disk_state(path: &std::path::Path, large: Option<&LargeFile>, bytes: &[u8]) -> String {
    if large.is_some() {
        file_metadata_state(path)
    } else {
        format!("{}:{}", file_metadata_state(path), digest(bytes))
    }
}

/// Whether the tab's own edits are unsaved: the buffer differs from the
/// document as of the last save/load. Deliberately NOT a comparison with the
/// file on disk — that fact has its own field (B183): an external change must
/// never read as the agent's unsaved work.
fn tab_dirty(tab: &Tab) -> bool {
    if tab.large_file.is_some() {
        return false;
    }
    digest(tab.document.bytes()) != tab.saved_digest
}

/// Whether the file on disk differs from what the tab last synced with.
fn tab_disk_diverged(tab: &Tab) -> bool {
    if let Some(large) = &tab.large_file {
        return disk_state(&tab.path, Some(large), &[]) != tab.disk_digest;
    }
    disk_state(&tab.path, None, tab.document.bytes()) != tab.disk_digest
}

/// Every payload key any handler reads. Requests carrying anything outside
/// this list are refused by name instead of having the extra silently
/// ignored (B180); keep in step with the client's flag table.
const KNOWN_PAYLOAD_KEYS: &[&str] = &[
    "acknowledge_force_save",
    "acknowledge_large_edit",
    "action",
    "after",
    "before",
    "bytes_base64",
    "column",
    "cursor_id",
    "delete_len",
    "detached",
    "file",
    "gradient",
    "granularity",
    "historical",
    "id",
    "job_id",
    "journal_action",
    "length",
    "limit",
    "line",
    "mode",
    "offset",
    "order",
    "owner",
    "page_lines",
    "pager_key",
    "presentation",
    "preserve_external",
    "progress",
    "query",
    "query_base64",
    "range_end_byte",
    "range_end_line",
    "range_start_byte",
    "range_start_line",
    "resume_token",
    "result",
    "backup_path",
    "target_path",
    "text",
    "visual",
    "wrap_width",
];

/// A large-edit refused before it ever touched the file must not leave its
/// job Queued forever (B194): fail it with the refusal reason.
fn fail_guarded_job(tab: &mut Tab, envelope: &ai_text_editor::protocol::Envelope, reason: &str) {
    if let Ok(id) = job_id(envelope) {
        let _ = tab.jobs.fail(id, reason);
    }
}

fn file_metadata_state(path: &std::path::Path) -> String {
    let Ok(metadata) = std::fs::metadata(path) else {
        return "missing".into();
    };
    let modified = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|value| format!("{}.{}", value.as_secs(), value.subsec_nanos()))
        .unwrap_or_else(|| "unknown".into());
    let readonly = metadata.permissions().readonly();
    #[cfg(unix)]
    let mode = {
        use std::os::unix::fs::PermissionsExt;
        metadata.permissions().mode()
    };
    #[cfg(not(unix))]
    let mode = if readonly { 0 } else { 1 };
    format!("{}:{}:{}:{}", metadata.len(), modified, mode, readonly)
}

fn page(envelope: &ai_text_editor::protocol::Envelope, tab: &mut Tab, frames: &mut Vec<Value>) {
    let key = envelope
        .payload
        .get("pager_key")
        .and_then(Value::as_str)
        .unwrap_or("");
    let offset = envelope
        .payload
        .get("offset")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let limit = envelope
        .payload
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(4) as usize;
    let historical_requested =
        envelope.payload.get("historical").and_then(Value::as_bool) == Some(true);
    let mut source_revision = tab.revision;
    let mut historical = false;
    let page = if let Some(results) = tab.results.get(key) {
        Some((
            results.len(),
            results
                .get(offset..offset.saturating_add(limit).min(results.len()))
                .unwrap_or(&[])
                .to_vec(),
        ))
    } else if let Ok(Some(page)) = tab
        .metadata
        .load_result_page(key, tab.revision, offset, limit)
    {
        Some(page)
    } else if historical_requested {
        match tab.metadata.load_historical_result_page(key, offset, limit) {
            Ok(Some((revision, count, matches))) => {
                source_revision = revision;
                historical = revision != tab.revision;
                Some((count, matches))
            }
            _ => None,
        }
    } else {
        None
    };
    let Some((count, matches)) = page else {
        frames.push(error(
            &envelope.request_id,
            "stale_result",
            "result set is absent or was invalidated by an edit; issue the search again",
        ));
        return;
    };
    frames.push(response(&envelope.request_id, json!({"pager_key": key, "offset": offset, "limit": limit, "count": count, "matches": matches, "complete": true, "generation": key.split(':').next().unwrap_or(""), "source_revision": source_revision, "stale": historical})));
}

fn cursor(envelope: &ai_text_editor::protocol::Envelope, tab: &mut Tab, frames: &mut Vec<Value>) {
    let id = envelope
        .payload
        .get("id")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let current = tab
        .cursors
        .get(&id)
        .copied()
        .unwrap_or(Position { line: 1, column: 0 });
    let wrap_width = envelope
        .payload
        .get("wrap_width")
        .and_then(Value::as_u64)
        .map(|width| width as usize);
    let next = if envelope.payload.get("visual").and_then(Value::as_bool) == Some(true) {
        let Some((line, column)) = envelope
            .payload
            .get("line")
            .and_then(Value::as_u64)
            .zip(envelope.payload.get("column").and_then(Value::as_u64))
        else {
            frames.push(error(
                &envelope.request_id,
                "visual_cursor_required",
                "--visual requires line and column",
            ));
            return;
        };
        let Some(text) = tab.document.text().ok() else {
            frames.push(error(
                &envelope.request_id,
                "visual_cursor_unsupported",
                "visual coordinates require a UTF-8 text document",
            ));
            return;
        };
        match navigation::logical_position(
            &text,
            Position {
                line: line as usize,
                column: column as usize,
            },
            wrap_width,
        ) {
            Ok(position) => clamp_cursor(tab, position),
            Err(message) => {
                frames.push(error(&envelope.request_id, "wrap_width_required", message));
                return;
            }
        }
    } else if let (Some(line), Some(column)) = (
        envelope.payload.get("line").and_then(Value::as_u64),
        envelope.payload.get("column").and_then(Value::as_u64),
    ) {
        clamp_cursor(
            tab,
            Position {
                line: line as usize,
                column: column as usize,
            },
        )
    } else {
        cursor_action(tab, current, envelope, frames)
    };
    tab.cursors.insert(id, next);
    let visual = wrap_width.and_then(|width| {
        tab.document
            .text()
            .ok()
            .and_then(|text| navigation::visual_position(&text, next, Some(width)).ok())
    });
    frames.push(response(
        &envelope.request_id,
        json!({"id": id, "line": next.line, "column": next.column, "visual": visual, "wrap_width": wrap_width, "cursors": tab.cursors}),
    ));
}

fn clamp_cursor(tab: &Tab, position: Position) -> Position {
    match tab.document.mode {
        DocumentMode::TextUtf8 => {
            navigation::clamp(&tab.document.text().unwrap_or_default(), position)
        }
        DocumentMode::RawBytes => {
            let bytes = tab.document.bytes();
            let line = position.line.max(1);
            let mut current_line = 1;
            let mut start = 0;
            for (offset, byte) in bytes.iter().enumerate() {
                if *byte == b'\n' {
                    if current_line == line {
                        return Position {
                            line,
                            column: position.column.min(offset - start),
                        };
                    }
                    current_line += 1;
                    start = offset + 1;
                }
            }
            if current_line == line {
                Position {
                    line,
                    column: position.column.min(bytes.len().saturating_sub(start)),
                }
            } else {
                Position {
                    line: current_line,
                    column: bytes.len().saturating_sub(start),
                }
            }
        }
        DocumentMode::HexView => {
            let bytes = tab.document.bytes();
            let max_line = (bytes.len() / 16).saturating_add(1).max(1);
            let line = position.line.max(1).min(max_line);
            let line_start = (line - 1) * 16;
            let line_len = bytes.len().saturating_sub(line_start).min(16);
            Position {
                line,
                column: position.column.min(line_len),
            }
        }
    }
}

fn cursor_action(
    tab: &Tab,
    current: Position,
    envelope: &ai_text_editor::protocol::Envelope,
    frames: &mut Vec<Value>,
) -> Position {
    let action = envelope
        .payload
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("");
    if tab.document.mode == DocumentMode::TextUtf8 {
        let text = tab.document.text().unwrap_or_default();
        return match action {
            "home" => navigation::home(&text, current),
            "end" => navigation::end(&text, current),
            "next_word" => navigation::next_word(&text, current),
            "previous_word" => navigation::previous_word(&text, current),
            "page_down" => navigation::page(
                &text,
                current,
                envelope
                    .payload
                    .get("page_lines")
                    .and_then(Value::as_u64)
                    .unwrap_or(40) as usize,
                1,
            ),
            "page_up" => navigation::page(
                &text,
                current,
                envelope
                    .payload
                    .get("page_lines")
                    .and_then(Value::as_u64)
                    .unwrap_or(40) as usize,
                -1,
            ),
            _ => {
                frames.push(error(
                    &envelope.request_id,
                    "invalid_cursor_action",
                    "use line/column or home, end, next_word, previous_word, page_up, page_down",
                ));
                return current;
            }
        };
    }
    let offset = position_offset(&tab.document, current);
    let next_offset = match action {
        "home" => match tab.document.mode {
            DocumentMode::HexView => (current.line.saturating_sub(1)) * 16,
            _ => line_start_offset(tab.document.bytes(), offset),
        },
        "end" => match tab.document.mode {
            DocumentMode::HexView => {
                ((current.line.saturating_sub(1)) * 16 + 16).min(tab.document.bytes().len())
            }
            _ => line_end_offset(tab.document.bytes(), offset),
        },
        "next_word" | "page_down" => offset
            .saturating_add(if action == "page_down" {
                envelope
                    .payload
                    .get("page_lines")
                    .and_then(Value::as_u64)
                    .unwrap_or(40) as usize
            } else {
                1
            })
            .min(tab.document.bytes().len()),
        "previous_word" | "page_up" => offset.saturating_sub(if action == "page_up" {
            envelope
                .payload
                .get("page_lines")
                .and_then(Value::as_u64)
                .unwrap_or(40) as usize
        } else {
            1
        }),
        _ => {
            frames.push(error(
                &envelope.request_id,
                "invalid_cursor_action",
                "use line/column or home, end, next_word, previous_word, page_up, page_down",
            ));
            return current;
        }
    };
    tab.document
        .coordinate(next_offset)
        .map(|coordinate| Position {
            line: coordinate.line,
            column: coordinate.column,
        })
        .unwrap_or(current)
}

fn line_start_offset(bytes: &[u8], offset: usize) -> usize {
    bytes[..offset.min(bytes.len())]
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map(|value| value + 1)
        .unwrap_or(0)
}

fn line_end_offset(bytes: &[u8], offset: usize) -> usize {
    bytes[offset.min(bytes.len())..]
        .iter()
        .position(|byte| *byte == b'\n')
        .map(|value| offset.min(bytes.len()) + value)
        .unwrap_or(bytes.len())
}

fn adjust_cursors(
    before: &Document,
    after: &Document,
    cursors: &mut BTreeMap<u64, Position>,
    offset: usize,
    delete_len: usize,
    replacement_len: usize,
) {
    let end = offset.saturating_add(delete_len);
    for position in cursors.values_mut() {
        let old_offset = position_offset(before, *position);
        let new_offset = if old_offset <= offset {
            old_offset
        } else if old_offset >= end {
            if replacement_len >= delete_len {
                old_offset.saturating_add(replacement_len - delete_len)
            } else {
                old_offset.saturating_sub(delete_len - replacement_len)
            }
        } else {
            offset.saturating_add(replacement_len)
        };
        *position = after
            .coordinate(new_offset.min(after.bytes().len()))
            .map(|coordinate| Position {
                line: coordinate.line,
                column: coordinate.column,
            })
            .unwrap_or(Position { line: 1, column: 0 });
    }
}

fn position_offset(document: &Document, position: Position) -> usize {
    match document.mode {
        DocumentMode::HexView => position
            .line
            .saturating_sub(1)
            .saturating_mul(16)
            .saturating_add(position.column)
            .min(document.bytes().len()),
        DocumentMode::RawBytes => {
            let mut line = 1usize;
            let mut offset = 0usize;
            for (index, byte) in document.bytes().iter().enumerate() {
                if line == position.line && offset == position.column {
                    return index;
                }
                if *byte == b'\n' {
                    if line == position.line {
                        return index;
                    }
                    line += 1;
                    offset = 0;
                } else {
                    offset += 1;
                }
            }
            document.bytes().len()
        }
        DocumentMode::TextUtf8 => {
            let text = document.text().unwrap_or_default();
            let mut current_line = 1usize;
            let mut line_start = 0usize;
            for (index, character) in text.char_indices() {
                if current_line == position.line
                    && text[line_start..index].chars().count() >= position.column
                {
                    return index;
                }
                if character == '\n' {
                    if current_line == position.line {
                        return index;
                    }
                    current_line += 1;
                    line_start = index + character.len_utf8();
                }
            }
            text.len()
        }
    }
}

fn save(envelope: &ai_text_editor::protocol::Envelope, tab: &mut Tab, frames: &mut Vec<Value>) {
    match fs::read(&tab.path) {
        Ok(bytes) if disk_state(&tab.path, None, &bytes) == tab.disk_digest => {}
        Ok(bytes) => {
            let byte_count = bytes.len();
            tab.pending_external = Some(bytes);
            frames.push(error_details(
                &envelope.request_id,
                "external_change",
                "file changed before save; resolve the external change first",
                json!({"bytes": byte_count, "choices": ["backup", "reload", "merge", "keep", "force_save"], "force_save_requires": "acknowledge_force_save=true"}),
            ));
            return;
        }
        // A tab opened on a path that did not exist yet has no disk copy to
        // reconcile against; the save creates the file. Any other read
        // error is a real failure.
        Err(error_value) if error_value.kind() == io::ErrorKind::NotFound => {}
        Err(error_value) => {
            frames.push(error(
                &envelope.request_id,
                "save_failed",
                format!("cannot save {}: {error_value}", tab.path.display()),
            ));
            return;
        }
    }
    let parent = tab
        .path
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let temp = unique_temp_path(parent, "save");
    if let Err(error_value) = write_atomic(&temp, &tab.path, tab.document.bytes()) {
        frames.push(error(
            &envelope.request_id,
            "save_failed",
            error_value.to_string(),
        ));
        return;
    }
    tab.disk_digest = disk_state(&tab.path, None, tab.document.bytes());
    tab.saved_digest = digest(tab.document.bytes());
    tab.base_bytes = tab.document.bytes().to_vec();
    journal_append(tab, "save", json!({"revision": tab.revision}));
    frames.push(response(
        &envelope.request_id,
        json!({"saved": true, "dirty": false, "revision": tab.revision, "bytes": tab.document.bytes().len()}),
    ));
}

fn save_as(envelope: &ai_text_editor::protocol::Envelope, tab: &mut Tab, frames: &mut Vec<Value>) {
    let Some(target) = envelope.payload.get("target_path").and_then(Value::as_str) else {
        frames.push(error(
            &envelope.request_id,
            "save_as_target_required",
            "target_path is required",
        ));
        return;
    };
    let target = PathBuf::from(target);
    if target.exists() {
        frames.push(error(&envelope.request_id, "save_as_target_exists", "save_as refuses to overwrite an existing path; choose a new path or an explicit force-save workflow"));
        return;
    }
    let parent = target.parent().unwrap_or_else(|| std::path::Path::new("."));
    if let Err(error_value) = fs::create_dir_all(parent) {
        frames.push(error(
            &envelope.request_id,
            "save_as_failed",
            format!("cannot save as {}: {error_value}", target.display()),
        ));
        return;
    }
    let temp = unique_temp_path(parent, "save-as");
    if let Err(error_value) = write_atomic(&temp, &target, tab.document.bytes()) {
        frames.push(error(
            &envelope.request_id,
            "save_as_failed",
            format!("cannot save as {}: {error_value}", target.display()),
        ));
        return;
    }
    journal_append(
        tab,
        "save_as",
        json!({"path": target, "revision": tab.revision}),
    );
    frames.push(response(
        &envelope.request_id,
        json!({"saved": true, "path": target, "revision": tab.revision, "active_path": tab.path}),
    ));
}

fn history_step(
    envelope: &ai_text_editor::protocol::Envelope,
    tab: &mut Tab,
    frames: &mut Vec<Value>,
    redo: bool,
) {
    if tab.large_file.is_some() {
        return large_history_step(envelope, tab, frames, redo);
    }
    let target = if redo {
        tab.history.redo_target()
    } else {
        tab.history.undo_target()
    }
    .map(ToOwned::to_owned);
    let Some(target) = target else {
        frames.push(error(
            &envelope.request_id,
            if redo {
                "nothing_to_redo"
            } else {
                "nothing_to_undo"
            },
            if redo {
                "redo history is empty"
            } else {
                "undo history is empty"
            },
        ));
        return;
    };
    let before = tab.document.clone();
    let mut after = before.clone();
    if let Err(error_value) = after.apply_bytes(0, before.bytes().len(), &target) {
        frames.push(error(
            &envelope.request_id,
            "history_refused",
            error_value.to_string(),
        ));
        return;
    }
    let revision = tab.revision.saturating_add(1);
    if let Err(error_value) = journal_append_result(
        tab,
        if redo { "redo" } else { "undo" },
        json!({
            "revision": revision,
            "before": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, before.bytes()),
            "after": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, after.bytes())
        }),
    ) {
        frames.push(error(
            &envelope.request_id,
            "journal_write_failed",
            error_value.to_string(),
        ));
        return;
    }
    let success = if redo {
        tab.history.redo(&mut tab.document)
    } else {
        tab.history.undo(&mut tab.document)
    };
    if !success {
        frames.push(error(
            &envelope.request_id,
            "history_commit_failed",
            "history changed while committing the journaled transition",
        ));
        return;
    }
    adjust_cursors(
        &before,
        &after,
        &mut tab.cursors,
        0,
        before.bytes().len(),
        after.bytes().len(),
    );
    tab.revision = revision;
    tab.results.clear();
    tab.index = LineIndex::build(tab.document.bytes(), tab.index.granularity);
    tab.index_complete = true;
    persist_index(tab);
    let _ = tab.metadata.record(
        &tab.path,
        tab.document.mode,
        revision,
        tab.document.bytes().len(),
    );
    frames.push(response(
        &envelope.request_id,
        json!({"revision": revision, "cursors": tab.cursors, "dirty": tab_dirty(tab), "disk_diverged": tab_disk_diverged(tab)}),
    ));
}

fn large_history_step(
    envelope: &ai_text_editor::protocol::Envelope,
    tab: &mut Tab,
    frames: &mut Vec<Value>,
    redo: bool,
) {
    let snapshot = if redo {
        tab.large_redo.pop()
    } else {
        tab.large_undo.pop()
    };
    let Some(snapshot) = snapshot else {
        frames.push(error(
            &envelope.request_id,
            if redo {
                "nothing_to_redo"
            } else {
                "nothing_to_undo"
            },
            if redo {
                "redo history is empty"
            } else {
                "undo history is empty"
            },
        ));
        return;
    };
    let Some(current) = tab.large_file.clone() else {
        frames.push(error(
            &envelope.request_id,
            "history_refused",
            "large-file state is unavailable",
        ));
        return;
    };
    let source = if redo {
        &snapshot.after
    } else {
        &snapshot.before
    };
    let target = if redo {
        &snapshot.before
    } else {
        &snapshot.after
    };
    if let Err(error_value) = current.restore_from(source) {
        if redo {
            tab.large_redo.push(snapshot);
        } else {
            tab.large_undo.push(snapshot);
        }
        frames.push(error(
            &envelope.request_id,
            "history_refused",
            error_value.to_string(),
        ));
        return;
    }
    let revision = tab.revision.saturating_add(1);
    if let Err(error_value) = journal_append_result(
        tab,
        if redo { "large_redo" } else { "large_undo" },
        json!({"revision": revision, "before_path": snapshot.before, "after_path": snapshot.after}),
    ) {
        let _ = current.restore_from(target);
        if redo {
            tab.large_redo.push(snapshot);
        } else {
            tab.large_undo.push(snapshot);
        }
        frames.push(error(
            &envelope.request_id,
            "journal_write_failed",
            error_value.to_string(),
        ));
        return;
    }
    let updated = LargeFile::open(&tab.path).unwrap_or(current);
    tab.disk_digest = disk_state(&tab.path, Some(&updated), &[]);
    tab.large_file = Some(updated.clone());
    tab.revision = revision;
    persist_index(tab);
    if redo {
        tab.large_undo.push(snapshot);
    } else {
        tab.large_redo.push(snapshot);
    }
    frames.push(response(
        &envelope.request_id,
        json!({"revision": revision, "large_file": true, "cursors": tab.cursors}),
    ));
}

fn close_tab(
    envelope: &ai_text_editor::protocol::Envelope,
    tab: &mut Tab,
    frames: &mut Vec<Value>,
) {
    let Some(action) = envelope
        .payload
        .get("journal_action")
        .and_then(Value::as_str)
    else {
        frames.push(error_details(
            &envelope.request_id,
            "journal_close_decision_required",
            "choose whether to preserve or clean the tab journal before closing",
            json!({"choices": ["preserve", "clean"], "journal": tab.journal.path(), "unsaved_changes": tab_dirty(tab)}),
        ));
        return;
    };
    if !matches!(action, "preserve" | "clean") {
        frames.push(error(
            &envelope.request_id,
            "invalid_journal_close_action",
            "journal_action must be preserve or clean",
        ));
        return;
    }
    if let Err(error_value) = journal_append_result(tab, "close", json!({"journal_action": action}))
    {
        frames.push(error(
            &envelope.request_id,
            "journal_write_failed",
            error_value.to_string(),
        ));
        return;
    }
    if action == "clean" {
        for snapshot in tab.large_undo.iter().chain(tab.large_redo.iter()) {
            let _ = fs::remove_file(&snapshot.before);
            let _ = fs::remove_file(&snapshot.after);
        }
        if let Err(error_value) = tab.journal.cleanup() {
            frames.push(error(
                &envelope.request_id,
                "journal_cleanup_failed",
                error_value.to_string(),
            ));
            return;
        }
        if let Err(error_value) = tab.metadata.cleanup() {
            frames.push(error(
                &envelope.request_id,
                "metadata_cleanup_failed",
                error_value.to_string(),
            ));
            return;
        }
    }
    if let Err(error_value) = session::unregister(&tab.session_token) {
        frames.push(error(
            &envelope.request_id,
            "session_cleanup_failed",
            error_value.to_string(),
        ));
        return;
    }
    tab.close_after_response = true;
    frames.push(response(
        &envelope.request_id,
        json!({"closed": true, "journal": action}),
    ));
}

fn resolve_external(
    envelope: &ai_text_editor::protocol::Envelope,
    tab: &mut Tab,
    frames: &mut Vec<Value>,
) {
    let action = envelope
        .payload
        .get("action")
        .and_then(Value::as_str)
        .unwrap_or("");
    let Some(external) = tab.pending_external.take() else {
        frames.push(error(
            &envelope.request_id,
            "no_external_change",
            "there is no pending external change",
        ));
        return;
    };
    if action == "backup" {
        if tab.large_file.is_some() {
            match write_large_external_backup(tab, envelope) {
                Ok(path) => {
                    tab.pending_external = Some(external);
                    frames.push(response(&envelope.request_id, json!({"resolved": "backup", "backup_path": path, "resolution_pending": true, "large_file": true, "revision": tab.revision})));
                }
                Err(error_value) => {
                    tab.pending_external = Some(external);
                    frames.push(error(&envelope.request_id, "backup_failed", error_value));
                }
            }
            return;
        }
        match write_external_backup(tab, envelope, &external) {
            Ok(path) => {
                tab.pending_external = Some(external);
                frames.push(response(&envelope.request_id, json!({"resolved": "backup", "backup_path": path, "resolution_pending": true, "revision": tab.revision})));
            }
            Err(error_value) => {
                tab.pending_external = Some(external);
                frames.push(error(&envelope.request_id, "backup_failed", error_value));
            }
        }
        return;
    }
    if let Some(file) = tab.large_file.clone() {
        if action == "keep" {
            tab.disk_digest = disk_state(&tab.path, Some(&file), &[]);
            frames.push(response(&envelope.request_id, json!({"resolved": "keep", "large_file": true, "save_required": false, "revision": tab.revision})));
        } else if action == "reload" {
            let updated = match LargeFile::open(&tab.path) {
                Ok(updated) => updated,
                Err(error_value) => {
                    tab.pending_external = Some(external);
                    frames.push(error(
                        &envelope.request_id,
                        "reload_failed",
                        error_value.to_string(),
                    ));
                    return;
                }
            };
            let revision = tab.revision.saturating_add(1);
            if let Err(error_value) = journal_append_result(
                tab,
                "external_reload",
                json!({"revision": revision, "large_file": true, "before": file.bytes, "after": updated.bytes}),
            ) {
                tab.pending_external = Some(external);
                frames.push(error(
                    &envelope.request_id,
                    "journal_write_failed",
                    error_value.to_string(),
                ));
                return;
            }
            tab.large_file = Some(updated.clone());
            tab.revision = revision;
            tab.disk_digest = disk_state(&tab.path, Some(&updated), &[]);
            tab.index = match updated.index_prefix(tab.index.granularity, DEFAULT_GRANULARITY) {
                Ok(index) => index,
                Err(error_value) => {
                    tab.pending_external = Some(external);
                    frames.push(error(
                        &envelope.request_id,
                        "index_failed",
                        error_value.to_string(),
                    ));
                    return;
                }
            };
            tab.index_complete = false;
            tab.index_loaded = false;
            persist_index(tab);
            let _ = tab.metadata.record(
                &tab.path,
                tab.document.mode,
                tab.revision,
                updated.bytes as usize,
            );
            frames.push(response(&envelope.request_id, json!({"resolved": "reload", "large_file": true, "revision": tab.revision, "history_event": "external_reload", "index_complete": false})));
        } else {
            tab.pending_external = Some(external);
            frames.push(error(&envelope.request_id, "large_file_resolution_requires_range", "merge and force_save require a bounded external range or an explicit large-file rewrite job; choose backup, reload, or keep for this alert"));
        }
        return;
    }
    match action {
        "reload" => {
            if let Err(error_value) = preserve_external(tab, envelope) {
                tab.pending_external = Some(external);
                frames.push(error(&envelope.request_id, "backup_failed", error_value));
                return;
            }
            let before = tab.document.clone();
            let mut after = before.clone();
            let length = before.bytes().len();
            if let Err(error_value) = after.apply_bytes(0, length, &external) {
                tab.pending_external = Some(external);
                frames.push(error(
                    &envelope.request_id,
                    "reload_refused",
                    error_value.to_string(),
                ));
                return;
            }
            let revision = tab.revision + 1;
            if let Err(error_value) = journal_append_result(
                tab,
                "external_reload",
                json!({
                    "revision": revision,
                    "before": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, before.bytes()),
                    "after": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, after.bytes())
                }),
            ) {
                tab.pending_external = Some(external);
                frames.push(error(
                    &envelope.request_id,
                    "journal_write_failed",
                    error_value.to_string(),
                ));
                return;
            }
            tab.document = after.clone();
            tab.history.record(&before, &after);
            tab.revision = revision;
            tab.base_bytes = external.clone();
            tab.disk_digest = disk_state(&tab.path, None, &external);
            tab.saved_digest = digest(tab.document.bytes());
            let _ = tab.metadata.record(
                &tab.path,
                tab.document.mode,
                tab.revision,
                tab.document.bytes().len(),
            );
            frames.push(response(&envelope.request_id, json!({"resolved": "reload", "revision": tab.revision, "history_event": "external_reload"})));
        }
        "keep" => {
            tab.disk_digest = disk_state(&tab.path, None, &external);
            tab.base_bytes = external;
            frames.push(response(
                &envelope.request_id,
                json!({"resolved": "keep", "save_required": true, "revision": tab.revision}),
            ));
        }
        "force_save"
            if envelope
                .payload
                .get("acknowledge_force_save")
                .and_then(Value::as_bool)
                == Some(true) =>
        {
            if let Err(error_value) = preserve_external(tab, envelope) {
                tab.pending_external = Some(external);
                frames.push(error(&envelope.request_id, "backup_failed", error_value));
                return;
            }
            let parent = tab
                .path
                .parent()
                .unwrap_or_else(|| std::path::Path::new("."));
            let latest = match fs::read(&tab.path) {
                Ok(bytes) => bytes,
                Err(error_value) => {
                    tab.pending_external = Some(external.clone());
                    frames.push(error(
                        &envelope.request_id,
                        "save_failed",
                        error_value.to_string(),
                    ));
                    return;
                }
            };
            if digest(&latest) != digest(&external) {
                tab.pending_external = Some(latest);
                frames.push(error(
                    &envelope.request_id,
                    "save_race_detected",
                    "the external file changed again; retry resolution",
                ));
                return;
            }
            let temp = unique_temp_path(parent, "force-save");
            if let Err(error_value) = write_atomic(&temp, &tab.path, tab.document.bytes()) {
                tab.pending_external = Some(external);
                frames.push(error(
                    &envelope.request_id,
                    "save_failed",
                    error_value.to_string(),
                ));
                return;
            }
            tab.disk_digest = disk_state(&tab.path, None, tab.document.bytes());
            tab.saved_digest = digest(tab.document.bytes());
            tab.base_bytes = tab.document.bytes().to_vec();
            frames.push(response(
                &envelope.request_id,
                json!({"resolved": "force_save", "saved": true, "revision": tab.revision}),
            ));
        }
        "force_save" => {
            tab.pending_external = Some(external);
            frames.push(error(
                &envelope.request_id,
                "force_save_ack_required",
                "set acknowledge_force_save=true after accepting loss of the external bytes",
            ));
        }
        "merge" => {
            let working = tab.document.bytes().to_vec();
            let merged = if working == tab.base_bytes {
                external.clone()
            } else if external == tab.base_bytes {
                working
            } else {
                tab.pending_external = Some(external);
                frames.push(error(
                    &envelope.request_id,
                    "merge_conflict",
                    "working and external changes overlap; resolve manually",
                ));
                return;
            };
            let before = tab.document.clone();
            let mut after = before.clone();
            let length = before.bytes().len();
            if let Err(error_value) = after.apply_bytes(0, length, &merged) {
                tab.pending_external = Some(external);
                frames.push(error(
                    &envelope.request_id,
                    "merge_refused",
                    error_value.to_string(),
                ));
                return;
            }
            let revision = tab.revision + 1;
            if let Err(error_value) = journal_append_result(
                tab,
                "external_merge",
                json!({
                    "revision": revision,
                    "before": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, before.bytes()),
                    "after": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, after.bytes())
                }),
            ) {
                tab.pending_external = Some(external);
                frames.push(error(
                    &envelope.request_id,
                    "journal_write_failed",
                    error_value.to_string(),
                ));
                return;
            }
            tab.document = after.clone();
            tab.history.record(&before, &after);
            tab.revision = revision;
            tab.disk_digest = disk_state(&tab.path, None, &external);
            tab.base_bytes = external;
            tab.results.clear();
            tab.index = LineIndex::build(tab.document.bytes(), tab.index.granularity);
            tab.index_complete = true;
            persist_index(tab);
            frames.push(response(&envelope.request_id, json!({"resolved": "merge", "revision": revision, "save_required": true, "history_event": "external_merge"})));
        }
        _ => {
            tab.pending_external = Some(external);
            frames.push(error(
                &envelope.request_id,
                "invalid_external_action",
                "choose action reload, merge, keep, or force_save",
            ));
        }
    }
}

fn preserve_external(
    tab: &Tab,
    envelope: &ai_text_editor::protocol::Envelope,
) -> Result<(), String> {
    if envelope
        .payload
        .get("preserve_external")
        .and_then(Value::as_bool)
        != Some(true)
    {
        return Ok(());
    }
    let external = tab
        .pending_external
        .as_ref()
        .ok_or_else(|| "no external bytes available".to_owned())?;
    write_external_backup(tab, envelope, external).map(|_| ())
}

fn write_external_backup(
    tab: &Tab,
    envelope: &ai_text_editor::protocol::Envelope,
    external: &[u8],
) -> Result<PathBuf, String> {
    let requested = envelope
        .payload
        .get("backup_path")
        .and_then(Value::as_str)
        .map(PathBuf::from);
    let backup = requested.unwrap_or_else(|| PathBuf::from(format!("{}.back", tab.path.display())));
    let parent = backup.parent().unwrap_or_else(|| std::path::Path::new("."));
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let temp = parent.join(format!(
        ".{}.ai-text-editor-back.tmp",
        backup
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("document")
    ));
    use std::fs::OpenOptions;
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|error| {
            if error.kind() == io::ErrorKind::AlreadyExists {
                "backup_exists".to_owned()
            } else {
                error.to_string()
            }
        })?;
    file.write_all(external)
        .map_err(|error| error.to_string())?;
    file.sync_all().map_err(|error| error.to_string())?;
    drop(file);
    if backup.exists() {
        let _ = fs::remove_file(&temp);
        return Err("backup_exists".to_owned());
    }
    fs::rename(&temp, &backup).map_err(|error| error.to_string())?;
    Ok(backup)
}

fn write_large_external_backup(
    tab: &Tab,
    envelope: &ai_text_editor::protocol::Envelope,
) -> Result<PathBuf, String> {
    let requested = envelope
        .payload
        .get("backup_path")
        .and_then(Value::as_str)
        .map(PathBuf::from);
    let backup = requested.unwrap_or_else(|| PathBuf::from(format!("{}.back", tab.path.display())));
    let parent = backup.parent().unwrap_or_else(|| std::path::Path::new("."));
    fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    let before = file_metadata_state(&tab.path);
    let temp = unique_temp_path(parent, "large-backup");
    let mut input = fs::File::open(&tab.path).map_err(|error| error.to_string())?;
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|error| error.to_string())?;
    if let Err(error_value) = io::copy(&mut input, &mut output) {
        let _ = fs::remove_file(&temp);
        return Err(error_value.to_string());
    }
    output.sync_all().map_err(|error| {
        let _ = fs::remove_file(&temp);
        error.to_string()
    })?;
    drop(output);
    if before != file_metadata_state(&tab.path) {
        let _ = fs::remove_file(&temp);
        return Err("backup_race_detected".into());
    }
    if backup.exists() {
        let _ = fs::remove_file(&temp);
        return Err("backup_exists".into());
    }
    fs::rename(&temp, &backup).map_err(|error| error.to_string())?;
    Ok(backup)
}

fn write_atomic(temp: &std::path::Path, target: &std::path::Path, bytes: &[u8]) -> io::Result<()> {
    let permissions = fs::metadata(target)
        .ok()
        .map(|metadata| metadata.permissions());
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(temp)?;
    file.write_all(bytes)?;
    if let Some(permissions) = permissions {
        file.set_permissions(permissions)?;
    }
    file.sync_all()?;
    drop(file);
    fs::rename(temp, target)
}

fn unique_temp_path(parent: &std::path::Path, purpose: &str) -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    parent.join(format!(
        ".ai-text-editor-{purpose}-{}-{stamp}.tmp",
        std::process::id()
    ))
}

fn copy_file_snapshot(source: &std::path::Path, target: &std::path::Path) -> io::Result<()> {
    let mut input = fs::File::open(source)?;
    let mut output = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(target)?;
    io::copy(&mut input, &mut output)?;
    if let Ok(metadata) = fs::metadata(source) {
        output.set_permissions(metadata.permissions())?;
    }
    output.sync_all()
}

fn search(envelope: &ai_text_editor::protocol::Envelope, tab: &mut Tab, frames: &mut Vec<Value>) {
    let mode_name = envelope.payload.get("mode").and_then(Value::as_str);
    let mode = match parse_mode(mode_name) {
        Ok(mode) => mode,
        Err(error_value) => {
            frames.push(error(
                &envelope.request_id,
                "search_mode_invalid",
                error_value.to_string(),
            ));
            return;
        }
    };
    let query = envelope
        .payload
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or("");
    let gradient = match envelope.payload.get("gradient") {
        None => None,
        Some(value) => match value.as_f64() {
            Some(value) => Some(value),
            None => {
                frames.push(error(
                    &envelope.request_id,
                    "search_invalid",
                    "gradient must be a JSON number from 0.0 through 1.0",
                ));
                return;
            }
        },
    };
    if mode == SearchMode::PathWildcard {
        let path = fs::canonicalize(&tab.path).unwrap_or_else(|_| tab.path.clone());
        let path_text = path.to_string_lossy().replace('\\', "/");
        match matches_with_gradient(mode, query, &path_text, gradient) {
            Ok(found) => {
                let results = found.into_iter().map(|(start, end)| json!({"path": path_text, "line": null, "column_start": null, "column_end": null, "matched": &path_text[start..end]})).collect();
                let result_id = search_result_id(envelope, tab.revision, mode_name, query);
                emit_results(envelope, tab, frames, result_id, results);
            }
            Err(error_value) => frames.push(error(
                &envelope.request_id,
                "search_invalid",
                error_value.to_string(),
            )),
        }
        return;
    }
    if mode != SearchMode::ExactBytes {
        if let Err(error_value) = matches_with_gradient(mode, query, "", gradient) {
            frames.push(error(
                &envelope.request_id,
                "search_invalid",
                error_value.to_string(),
            ));
            return;
        }
    }
    if let Some(file) = tab.large_file.clone() {
        if mode == SearchMode::ExactBytes {
            let encoded = envelope
                .payload
                .get("query_base64")
                .and_then(Value::as_str)
                .unwrap_or(query);
            let bytes =
                match base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded) {
                    Ok(bytes) => bytes,
                    Err(error_value) => {
                        frames.push(error(
                            &envelope.request_id,
                            "invalid_base64",
                            error_value.to_string(),
                        ));
                        return;
                    }
                };
            let Some(start) = envelope
                .payload
                .get("range_start_byte")
                .and_then(Value::as_u64)
            else {
                frames.push(error(
                    &envelope.request_id,
                    "large_search_range_required",
                    "large byte searches require range_start_byte and range_end_byte",
                ));
                return;
            };
            let Some(end) = envelope
                .payload
                .get("range_end_byte")
                .and_then(Value::as_u64)
            else {
                frames.push(error(
                    &envelope.request_id,
                    "large_search_range_required",
                    "large byte searches require range_start_byte and range_end_byte",
                ));
                return;
            };
            let Ok(length) = usize::try_from(end.saturating_sub(start)) else {
                frames.push(error(
                    &envelope.request_id,
                    "large_search_range_invalid",
                    "byte search range is too large",
                ));
                return;
            };
            if end < start
                || end > file.bytes
                || length > ai_text_editor::large_file::DEFAULT_READ_BYTES
            {
                frames.push(error(
                    &envelope.request_id,
                    "large_search_range_invalid",
                    "byte search range must be ordered, within the file, and no larger than the bounded read limit",
                ));
                return;
            }
            let result_id = search_result_id(envelope, tab.revision, Some("exact_bytes"), encoded);
            let query_digest = blake3::hash(encoded.as_bytes()).to_hex().to_string();
            emit_large_results(
                envelope,
                tab,
                frames,
                LargeResultSpec {
                    result_id,
                    mode: "exact_bytes".into(),
                    query_digest,
                    search_range: json!({"start_byte": start, "end_byte": end}),
                },
                |emit| {
                    file.search_bytes_each(&bytes, start, length, |(found_start, found_end, contents)| {
                        emit(json!({
                            "byte_start": found_start,
                            "byte_end": found_end,
                            "contents_base64": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, contents)
                        }))
                    })
                },
            );
            return;
        }
        let start_line = envelope
            .payload
            .get("range_start_line")
            .and_then(Value::as_u64)
            .unwrap_or(1);
        let end_line = envelope
            .payload
            .get("range_end_line")
            .and_then(Value::as_u64);
        let Some(end_line) = end_line else {
            frames.push(error_details(
                &envelope.request_id,
                "large_search_range_required",
                "large-file searches require an explicit inclusive line range",
                json!({"range_start_line": start_line, "range_end_line": null, "choices": ["provide range_end_line", "build or inspect the index", "use a bounded read or agent-owned job"]}),
            ));
            return;
        };
        if start_line == 0 || end_line < start_line {
            frames.push(error(
                &envelope.request_id,
                "large_search_range_invalid",
                "range_start_line must be positive and range_end_line must not precede it",
            ));
            return;
        }
        let result_id = search_result_id(envelope, tab.revision, mode_name, query);
        let query_digest = blake3::hash(query.as_bytes()).to_hex().to_string();
        let block = tab.index.block_for_line(start_line as usize).clone();
        emit_large_results(
            envelope,
            tab,
            frames,
            LargeResultSpec {
                result_id,
                mode: mode_name.unwrap_or("unknown").into(),
                query_digest,
                search_range: json!({"start_line": start_line, "end_line": end_line}),
            },
            |emit| {
                file.search_text_mode_each_from(
                    mode,
                    query,
                    LineSearchWindow {
                        start_line,
                        end_line,
                        start_offset: block.byte_offset as u64,
                        indexed_line: block.line,
                    },
                    gradient,
                    |(line, start, end, contents)| {
                        emit(json!({"line": line, "column_start": start, "column_end": end, "contents": contents}))
                    },
                )
            },
        );
        return;
    }
    if mode == SearchMode::ExactBytes {
        let encoded = envelope
            .payload
            .get("query_base64")
            .and_then(Value::as_str)
            .unwrap_or(query);
        let bytes =
            match base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded) {
                Ok(bytes) => bytes,
                Err(error_value) => {
                    frames.push(error(
                        &envelope.request_id,
                        "invalid_base64",
                        error_value.to_string(),
                    ));
                    return;
                }
            };
        let found_matches: Vec<Value> = find_bytes(&bytes, tab.document.bytes())
            .into_iter()
            .map(|(start, end)| {
                let coordinate = tab.document.coordinate(start).unwrap();
                let end_coordinate = tab.document.coordinate(end).unwrap();
                json!({
                    "line": coordinate.line,
                    "column_start": coordinate.column,
                    "column_end": end_coordinate.column,
                    "end_line": end_coordinate.line,
                    "byte_start": start,
                    "byte_end": end,
                    "contents_base64": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &tab.document.bytes()[start..end])
                })
            })
            .collect();
        let result_id = search_result_id(envelope, tab.revision, Some("exact_bytes"), encoded);
        emit_results(envelope, tab, frames, result_id, found_matches);
        return;
    }
    match tab.document.text() {
        Ok(text) => {
            let mut found_matches = Vec::new();
            let mut line_start = 0;
            for line in text.split_inclusive('\n') {
                let content = line.strip_suffix('\n').unwrap_or(line);
                match matches_with_gradient(mode, query, content, gradient) {
                    Ok(ranges) => {
                        for (start, end) in ranges {
                            let coordinate =
                                text_coordinate(&tab.document, &text, line_start + start)
                                    .unwrap_or(ai_text_editor::document::Coordinate {
                                        line: 1,
                                        column: 0,
                                    });
                            found_matches.push(json!({"line": coordinate.line, "column_start": coordinate.column, "column_end": coordinate.column + content[start..end].chars().count(), "contents": &content[start..end]}));
                        }
                    }
                    Err(error_value) => {
                        frames.push(error(
                            &envelope.request_id,
                            "search_invalid",
                            error_value.to_string(),
                        ));
                        return;
                    }
                }
                line_start += line.len();
            }
            let result_id = search_result_id(envelope, tab.revision, mode_name, query);
            emit_results(envelope, tab, frames, result_id, found_matches);
        }
        Err(error_value) => frames.push(error(
            &envelope.request_id,
            "invalid_utf8",
            error_value.to_string(),
        )),
    }
}

fn text_coordinate(
    document: &Document,
    text: &str,
    offset: usize,
) -> Option<ai_text_editor::document::Coordinate> {
    if !document.normalize_nfc {
        return document.coordinate(offset).ok();
    }
    let prefix = text.get(..offset)?;
    Some(ai_text_editor::document::Coordinate {
        line: prefix.matches('\n').count() + 1,
        column: prefix
            .rsplit('\n')
            .next()
            .unwrap_or_default()
            .chars()
            .count(),
    })
}

fn search_result_id(
    envelope: &ai_text_editor::protocol::Envelope,
    revision: u64,
    mode: Option<&str>,
    query: &str,
) -> String {
    let identity = json!({
        "mode": mode,
        "query": query,
        "parameters": envelope.payload,
    });
    format!(
        "{}:{}:{}",
        revision,
        mode.unwrap_or("unknown"),
        blake3::hash(&ai_text_editor::protocol::canonical_json(&identity)).to_hex()
    )
}

fn emit_results(
    envelope: &ai_text_editor::protocol::Envelope,
    tab: &mut Tab,
    frames: &mut Vec<Value>,
    result_id: String,
    mut results: Vec<Value>,
) {
    if let Some(start) = envelope
        .payload
        .get("range_start_line")
        .and_then(Value::as_u64)
    {
        results.retain(|result| result.get("line").and_then(Value::as_u64).unwrap_or(0) >= start);
    }
    if let Some(end) = envelope
        .payload
        .get("range_end_line")
        .and_then(Value::as_u64)
    {
        results.retain(|result| {
            result
                .get("line")
                .and_then(Value::as_u64)
                .unwrap_or(u64::MAX)
                <= end
        });
    }
    if envelope.payload.get("order").and_then(Value::as_str) == Some("reverse") {
        results.reverse();
    }
    let count = results.len();
    let preview = envelope
        .payload
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(4) as usize;
    let visible = results.iter().take(preview).cloned().collect::<Vec<_>>();
    let mode = envelope
        .payload
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let query_digest = blake3::hash(
        envelope
            .payload
            .get("query")
            .and_then(Value::as_str)
            .unwrap_or("")
            .as_bytes(),
    )
    .to_hex()
    .to_string();
    let _ = tab.metadata.record_result_matches(
        &result_id,
        mode,
        &query_digest,
        tab.revision,
        &results,
        true,
    );
    tab.results.insert(result_id.clone(), results);
    frames.push(response(
        &envelope.request_id,
        json!({
            "result_id": result_id,
            "count": count,
            "pager_key": result_id,
            "matches": visible,
            "returned": preview.min(count),
            "complete": true,
            "search_range": {
                "start_line": envelope.payload.get("range_start_line").and_then(Value::as_u64),
                "end_line": envelope.payload.get("range_end_line").and_then(Value::as_u64),
                "start_byte": envelope.payload.get("range_start_byte").and_then(Value::as_u64),
                "end_byte": envelope.payload.get("range_end_byte").and_then(Value::as_u64)
            }
        }),
    ));
}

struct LargeResultSpec {
    result_id: String,
    mode: String,
    query_digest: String,
    search_range: Value,
}

fn emit_large_results<F>(
    envelope: &ai_text_editor::protocol::Envelope,
    tab: &mut Tab,
    frames: &mut Vec<Value>,
    spec: LargeResultSpec,
    scan: F,
) where
    F: FnOnce(&mut dyn FnMut(Value) -> io::Result<()>) -> io::Result<usize>,
{
    if let Err(error_value) = tab.metadata.begin_result(
        &spec.result_id,
        &spec.mode,
        &spec.query_digest,
        tab.revision,
    ) {
        frames.push(error(
            &envelope.request_id,
            "result_persist_failed",
            error_value.to_string(),
        ));
        return;
    }
    let preview_limit = envelope
        .payload
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(4) as usize;
    let mut preview = Vec::new();
    let mut chunk = Vec::new();
    let mut append = |value: Value| -> io::Result<()> {
        if preview.len() < preview_limit {
            preview.push(value.clone());
        }
        chunk.push(value);
        if chunk.len() >= 256 {
            tab.metadata
                .append_result_matches(&spec.result_id, &chunk)?;
            chunk.clear();
        }
        Ok(())
    };
    if let Err(error_value) = scan(&mut append) {
        frames.push(error(
            &envelope.request_id,
            if error_value.kind() == io::ErrorKind::InvalidInput {
                "search_invalid"
            } else {
                "large_search_failed"
            },
            error_value.to_string(),
        ));
        return;
    }
    if let Err(error_value) = tab.metadata.append_result_matches(&spec.result_id, &chunk) {
        frames.push(error(
            &envelope.request_id,
            "result_persist_failed",
            error_value.to_string(),
        ));
        return;
    }
    let count = match tab.metadata.finish_result(&spec.result_id) {
        Ok(count) => count,
        Err(error_value) => {
            frames.push(error(
                &envelope.request_id,
                "result_persist_failed",
                error_value.to_string(),
            ));
            return;
        }
    };
    tab.results.remove(&spec.result_id);
    let result_id = spec.result_id;
    frames.push(response(
        &envelope.request_id,
        json!({
            "result_id": result_id.clone(),
            "count": count,
            "pager_key": result_id,
            "matches": preview,
            "returned": preview.len(),
            "complete": true,
            "search_range": spec.search_range
        }),
    ));
}

fn job_start(
    envelope: &ai_text_editor::protocol::Envelope,
    tab: &mut Tab,
    frames: &mut Vec<Value>,
) {
    let owner = envelope
        .payload
        .get("owner")
        .and_then(Value::as_str)
        .unwrap_or("client");
    let detached = envelope
        .payload
        .get("detached")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let snapshot = tab.jobs.start(owner, detached);
    frames.push(response(&envelope.request_id, json!({"job": snapshot})));
}

fn large_edit(
    envelope: &ai_text_editor::protocol::Envelope,
    tab: &mut Tab,
    frames: &mut Vec<Value>,
) {
    if envelope
        .payload
        .get("acknowledge_large_edit")
        .and_then(Value::as_bool)
        != Some(true)
    {
        frames.push(error(
            &envelope.request_id,
            "large_edit_ack_required",
            "set acknowledge_large_edit=true after confirming the streamed rewrite cost",
        ));
        return;
    }
    let Some(file) = tab.large_file.clone() else {
        frames.push(error(
            &envelope.request_id,
            "not_a_large_file",
            "large_edit is only for bounded large-file tabs",
        ));
        return;
    };
    let id = match job_id(envelope) {
        Ok(id) => id,
        Err(message) => return job_error(envelope, frames, "invalid_job", message),
    };
    let Some(token) = job_token(envelope) else {
        return job_error(
            envelope,
            frames,
            "job_unauthorized",
            "large_edit requires the job's resume_token",
        );
    };
    if let Err(error_value) = tab.jobs.get(id, Some(token)) {
        return job_error(
            envelope,
            frames,
            "job_unauthorized",
            error_value.to_string(),
        );
    }
    let offset = envelope
        .payload
        .get("offset")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let delete_len = envelope
        .payload
        .get("delete_len")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let replacement = match replacement_bytes(envelope) {
        Ok(bytes) => bytes,
        Err((code, message)) => {
            return job_error(envelope, frames, code, message);
        }
    };
    if replacement.len() > ai_text_editor::large_file::DEFAULT_READ_BYTES {
        return job_error(
            envelope,
            frames,
            "large_edit_replacement_too_large",
            "replacement exceeds the bounded large-edit payload limit",
        );
    }
    if let Err(error_value) = tab.jobs.running(id) {
        return job_error(
            envelope,
            frames,
            "job_start_failed",
            error_value.to_string(),
        );
    }
    let history_root = tab
        .journal
        .path()
        .parent()
        .unwrap_or_else(|| std::path::Path::new("."));
    let history_id = format!("{}-{}", tab.journal_seq.saturating_add(1), id);
    let before_path = history_root.join(format!("large-history-{history_id}-before"));
    let after_path = history_root.join(format!("large-history-{history_id}-after"));
    if let Err(error_value) = copy_file_snapshot(&tab.path, &before_path) {
        let _ = tab.jobs.fail(id, error_value.to_string());
        return job_error(
            envelope,
            frames,
            "large_history_snapshot_failed",
            error_value.to_string(),
        );
    }
    match file.rewrite(offset, delete_len, &replacement) {
        Ok(updated) => {
            if let Err(error_value) = copy_file_snapshot(&tab.path, &after_path) {
                let _ = file.restore_from(&before_path);
                let _ = tab.jobs.fail(id, error_value.to_string());
                return job_error(
                    envelope,
                    frames,
                    "large_history_snapshot_failed",
                    error_value.to_string(),
                );
            }
            let revision = tab.revision.saturating_add(1);
            if let Err(error_value) = journal_append_result(
                tab,
                "large_edit",
                json!({"offset": offset, "delete_len": delete_len, "replacement_bytes": replacement.len(), "revision": revision, "before_path": before_path, "after_path": after_path}),
            ) {
                let _ = file.restore_from(&before_path);
                let _ = tab.jobs.fail(id, error_value.to_string());
                return job_error(
                    envelope,
                    frames,
                    "journal_write_failed",
                    error_value.to_string(),
                );
            }
            tab.disk_digest = disk_state(&tab.path, Some(&updated), &[]);
            tab.base_bytes.clear();
            tab.large_file = Some(updated.clone());
            tab.pending_external = None;
            tab.revision = revision;
            tab.index = updated
                .index(tab.index.granularity)
                .unwrap_or_else(|_| LineIndex::build(&[], tab.index.granularity));
            tab.index_complete = true;
            persist_index(tab);
            let _ = tab.metadata.record(
                &tab.path,
                tab.document.mode,
                tab.revision,
                updated.bytes as usize,
            );
            tab.large_undo.push(LargeHistory {
                before: before_path,
                after: after_path,
            });
            tab.large_redo.clear();
            match tab.jobs.complete(
                id,
                vec![json!({"saved": true, "bytes": updated.bytes, "revision": tab.revision})],
            ) {
                Ok(snapshot) => frames.push(response(
                    &envelope.request_id,
                    json!({"job": snapshot, "revision": tab.revision, "cursors": tab.cursors}),
                )),
                Err(error_value) => job_error(
                    envelope,
                    frames,
                    "job_complete_failed",
                    error_value.to_string(),
                ),
            }
        }
        Err(error_value) => {
            let message = error_value.to_string();
            let _ = tab.jobs.fail(id, message.clone());
            job_error(envelope, frames, "large_edit_failed", message);
        }
    }
}

fn replacement_bytes(
    envelope: &ai_text_editor::protocol::Envelope,
) -> Result<Vec<u8>, (&'static str, String)> {
    if let Some(encoded) = envelope.payload.get("bytes_base64").and_then(Value::as_str) {
        return base64::Engine::decode(&base64::engine::general_purpose::STANDARD, encoded)
            .map_err(|error| ("invalid_base64", error.to_string()));
    }
    Ok(envelope
        .payload
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or("")
        .as_bytes()
        .to_vec())
}

fn job_id(envelope: &ai_text_editor::protocol::Envelope) -> Result<u64, String> {
    envelope
        .payload
        .get("job_id")
        .and_then(Value::as_u64)
        .ok_or_else(|| "job_id is required".to_owned())
}

fn job_token(envelope: &ai_text_editor::protocol::Envelope) -> Option<&str> {
    envelope.payload.get("resume_token").and_then(Value::as_str)
}

fn job_error(
    envelope: &ai_text_editor::protocol::Envelope,
    frames: &mut Vec<Value>,
    code: &str,
    message: impl Into<String>,
) {
    frames.push(error(&envelope.request_id, code, message.into()));
}

/// B182: every verb that reads or moves a job requires the owner's
/// resume_token, and it must match. Polling without it used to disclose the
/// token to any client on the endpoint — job ids are sequential integers.
fn authorized_job(
    envelope: &ai_text_editor::protocol::Envelope,
    tab: &mut Tab,
    frames: &mut Vec<Value>,
) -> Option<u64> {
    let id = match job_id(envelope) {
        Ok(id) => id,
        Err(message) => {
            job_error(envelope, frames, "invalid_job", message);
            return None;
        }
    };
    let Some(token) = job_token(envelope) else {
        job_error(
            envelope,
            frames,
            "job_unauthorized",
            "job operations require the owner's resume_token",
        );
        return None;
    };
    match tab.jobs.get(id, Some(token)) {
        Ok(_) => Some(id),
        Err(error_value) => {
            job_error(
                envelope,
                frames,
                "job_unauthorized",
                error_value.to_string(),
            );
            None
        }
    }
}

fn job_poll(envelope: &ai_text_editor::protocol::Envelope, tab: &mut Tab, frames: &mut Vec<Value>) {
    let Some(id) = authorized_job(envelope, tab, frames) else {
        return;
    };
    match tab.jobs.get(id, job_token(envelope)) {
        Ok(snapshot) => frames.push(response(&envelope.request_id, json!({"job": snapshot}))),
        Err(error_value) => job_error(envelope, frames, "job_unavailable", error_value.to_string()),
    }
}

fn job_progress(
    envelope: &ai_text_editor::protocol::Envelope,
    tab: &mut Tab,
    frames: &mut Vec<Value>,
) {
    let Some(id) = authorized_job(envelope, tab, frames) else {
        return;
    };
    let progress = envelope
        .payload
        .get("progress")
        .cloned()
        .unwrap_or(Value::Null);
    match tab.jobs.progress(id, progress) {
        Ok(snapshot) => frames.push(response(&envelope.request_id, json!({"job": snapshot}))),
        Err(error_value) => job_error(
            envelope,
            frames,
            "job_update_failed",
            error_value.to_string(),
        ),
    }
}

fn job_complete(
    envelope: &ai_text_editor::protocol::Envelope,
    tab: &mut Tab,
    frames: &mut Vec<Value>,
) {
    let Some(id) = authorized_job(envelope, tab, frames) else {
        return;
    };
    // B181: a result that is not an array is wrapped, never silently emptied.
    let result = match envelope.payload.get("result") {
        None | Some(Value::Null) => Vec::new(),
        Some(Value::Array(items)) => items.clone(),
        Some(value) => vec![value.clone()],
    };
    match tab.jobs.complete(id, result) {
        Ok(snapshot) => frames.push(response(&envelope.request_id, json!({"job": snapshot}))),
        Err(error_value) => job_error(
            envelope,
            frames,
            "job_complete_failed",
            error_value.to_string(),
        ),
    }
}

fn job_cancel(
    envelope: &ai_text_editor::protocol::Envelope,
    tab: &mut Tab,
    frames: &mut Vec<Value>,
) {
    let Some(id) = authorized_job(envelope, tab, frames) else {
        return;
    };
    match tab.jobs.cancel(id) {
        Ok(snapshot) => frames.push(response(&envelope.request_id, json!({"job": snapshot}))),
        Err(error_value) => job_error(
            envelope,
            frames,
            "job_cancel_failed",
            error_value.to_string(),
        ),
    }
}

fn job_transfer(
    envelope: &ai_text_editor::protocol::Envelope,
    tab: &mut Tab,
    frames: &mut Vec<Value>,
) {
    let id = match job_id(envelope) {
        Ok(id) => id,
        Err(message) => return job_error(envelope, frames, "invalid_job", message),
    };
    let Some(token) = job_token(envelope) else {
        return job_error(envelope, frames, "invalid_job", "resume_token is required");
    };
    let owner = envelope
        .payload
        .get("owner")
        .and_then(Value::as_str)
        .unwrap_or("client");
    match tab.jobs.transfer(id, token, owner) {
        Ok(snapshot) => frames.push(response(&envelope.request_id, json!({"job": snapshot}))),
        Err(error_value) => job_error(
            envelope,
            frames,
            "job_transfer_failed",
            error_value.to_string(),
        ),
    }
}

fn job_release(
    envelope: &ai_text_editor::protocol::Envelope,
    tab: &mut Tab,
    frames: &mut Vec<Value>,
) {
    let id = match job_id(envelope) {
        Ok(id) => id,
        Err(message) => return job_error(envelope, frames, "invalid_job", message),
    };
    let Some(token) = job_token(envelope) else {
        return job_error(envelope, frames, "invalid_job", "resume_token is required");
    };
    match tab.jobs.release(id, token) {
        Ok(snapshot) => frames.push(response(&envelope.request_id, json!({"job": snapshot}))),
        Err(error_value) => job_error(
            envelope,
            frames,
            "job_release_failed",
            error_value.to_string(),
        ),
    }
}

fn die(message: &str) -> ! {
    eprintln!("ai-text-editor-server: {message}");
    std::process::exit(64);
}

/// Whether the endpoint file at `discovery` names a server process that is
/// demonstrably gone. A `kill`ed server leaves its endpoint and socket
/// behind; refusing a takeover then protects nothing and wedges every later
/// `open` (the refusal message names a flag for a problem the agent was
/// never supposed to see). Only a *proven* dead pid takes over silently —
/// an unknown owner keeps the explicit `--takeover-stale-endpoint` gate.
#[cfg(unix)]
fn endpoint_owner_is_gone(discovery: &std::path::Path) -> bool {
    match read_endpoint_metadata(discovery) {
        Ok(metadata) => metadata.pid.is_some_and(|pid| !pid_is_alive(pid)),
        Err(_) => false,
    }
}

#[cfg(unix)]
fn pid_is_alive(pid: u32) -> bool {
    // signal 0 performs the existence and permission checks without
    // touching the process; EPERM means it exists and is not ours to kill.
    let result = unsafe { libc::kill(pid as libc::c_int, 0) };
    result == 0 || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
}
