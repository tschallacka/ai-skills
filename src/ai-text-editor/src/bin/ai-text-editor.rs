// MODE: DEV
// PACKAGE: PROD
use ai_text_editor::protocol::Envelope;
use ai_text_editor::transport::{
    endpoint_for_file, read_endpoint, read_session, request, write_session, Endpoint,
};
use serde_json::{json, Value};
use std::path::PathBuf;

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
    let file = option(&args, "--file").map(PathBuf::from);
    let presentation = option(&args, "--presentation").unwrap_or_else(|| "structured".into());
    let session_path = option(&args, "--session-token")
        .map(PathBuf::from)
        .or_else(|| session_identity(&args).map(|id| session_path(&id)));
    let (endpoint, saved_auth_token, saved_session_token) = match option(&args, "--endpoint") {
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
            let token = session_path.as_ref().unwrap();
            let session = read_session(token).unwrap_or_else(|error| {
                die(&format!(
                    "cannot read session token {}: {error}",
                    token.display()
                ))
            });
            (session.endpoint, session.auth_token, session.session_token)
        }
        None if option(&args, "--session-token").is_some() => die(&format!(
            "explicit session token {} does not exist; provide a valid token or --endpoint",
            session_path.as_ref().unwrap().display()
        )),
        None => {
            let file = file
                .as_ref()
                .unwrap_or_else(|| die("--file or --endpoint is required"));
            (
                read_endpoint(&endpoint_for_file(file)).unwrap_or_else(|error| {
                    die(&format!(
                        "no server discovered for {}: {error}",
                        file.display()
                    ))
                }),
                None,
                None,
            )
        }
    };
    let mut payload = serde_json::Map::new();
    if let Some(value) = option(&args, "--bytes-base64") {
        payload.insert("bytes_base64".into(), Value::String(value));
    }
    if let Some(value) = option(&args, "--id") {
        payload.insert("id".into(), json!(parse_number(&value)));
    }
    if let Some(value) = option(&args, "--line") {
        payload.insert("line".into(), json!(parse_number(&value)));
    }
    if let Some(value) = option(&args, "--column") {
        payload.insert("column".into(), json!(parse_number(&value)));
    }
    if let Some(value) = option(&args, "--action") {
        payload.insert("action".into(), Value::String(value));
    }
    if let Some(value) = option(&args, "--page-lines") {
        payload.insert("page_lines".into(), json!(parse_number(&value)));
    }
    if let Some(value) = option(&args, "--wrap-width") {
        payload.insert("wrap_width".into(), json!(parse_number(&value)));
    }
    if args.iter().any(|arg| arg == "--visual") {
        payload.insert("visual".into(), Value::Bool(true));
    }
    for name in ["--before", "--after"] {
        if let Some(value) = option(&args, name) {
            payload.insert(
                name.trim_start_matches("--").into(),
                json!(parse_number(&value)),
            );
        }
    }
    for (argument, field) in [
        ("--range-start-line", "range_start_line"),
        ("--range-end-line", "range_end_line"),
        ("--range-start-byte", "range_start_byte"),
        ("--range-end-byte", "range_end_byte"),
    ] {
        if let Some(value) = option(&args, argument) {
            payload.insert(field.into(), json!(parse_number(&value)));
        }
    }
    if let Some(value) = option(&args, "--order") {
        payload.insert("order".into(), Value::String(value));
    }
    if let Some(value) = option(&args, "--gradient") {
        payload.insert(
            "gradient".into(),
            json!(value
                .parse::<f64>()
                .unwrap_or_else(|_| die("--gradient must be a number"))),
        );
    }
    let expected_revision = option(&args, "--expected-revision").map(|value| parse_number(&value));
    if let Some(value) = option(&args, "--offset") {
        payload.insert("offset".into(), json!(parse_number(&value)));
    }
    if let Some(value) = option(&args, "--length") {
        payload.insert("length".into(), json!(parse_number(&value)));
    }
    if let Some(value) = option(&args, "--granularity") {
        payload.insert("granularity".into(), json!(parse_number(&value)));
    }
    if let Some(value) = option(&args, "--pager-key") {
        payload.insert("pager_key".into(), Value::String(value));
    }
    if let Some(value) = option(&args, "--limit") {
        payload.insert("limit".into(), json!(parse_number(&value)));
    }
    if let Some(value) = option(&args, "--delete-len") {
        payload.insert("delete_len".into(), json!(parse_number(&value)));
    }
    if let Some(value) = option(&args, "--text") {
        payload.insert("text".into(), Value::String(value));
    }
    if let Some(value) = option(&args, "--query") {
        payload.insert("query".into(), Value::String(value));
    }
    if let Some(value) = option(&args, "--query-base64") {
        payload.insert("query_base64".into(), Value::String(value));
    }
    if args.iter().any(|arg| arg == "--acknowledge-force-save") {
        payload.insert("acknowledge_force_save".into(), Value::Bool(true));
    }
    if args.iter().any(|arg| arg == "--acknowledge-large-edit") {
        payload.insert("acknowledge_large_edit".into(), Value::Bool(true));
    }
    if args.iter().any(|arg| arg == "--preserve-external") {
        payload.insert("preserve_external".into(), Value::Bool(true));
    }
    if let Some(value) = option(&args, "--backup-path") {
        payload.insert("backup_path".into(), Value::String(value));
    }
    if let Some(value) = option(&args, "--target-path") {
        payload.insert("target_path".into(), Value::String(value));
    }
    if let Some(value) = option(&args, "--journal-action") {
        payload.insert("journal_action".into(), Value::String(value));
    }
    for (argument, field) in [
        ("--job-id", "job_id"),
        ("--cursor-id", "cursor_id"),
        ("--owner", "owner"),
        ("--resume-token", "resume_token"),
    ] {
        if let Some(value) = option(&args, argument) {
            if field == "job_id" || field == "cursor_id" {
                payload.insert(field.into(), json!(parse_number(&value)));
            } else {
                payload.insert(field.into(), Value::String(value));
            }
        }
    }
    if let Some(value) = option(&args, "--progress-json") {
        payload.insert(
            "progress".into(),
            serde_json::from_str(&value)
                .unwrap_or_else(|error| die(&format!("invalid --progress-json: {error}"))),
        );
    }
    if let Some(value) = option(&args, "--result-json") {
        payload.insert(
            "result".into(),
            serde_json::from_str(&value)
                .unwrap_or_else(|error| die(&format!("invalid --result-json: {error}"))),
        );
    }
    if args.iter().any(|arg| arg == "--detached") {
        payload.insert("detached".into(), Value::Bool(true));
    }
    if args.iter().any(|arg| arg == "--historical") {
        payload.insert("historical".into(), Value::Bool(true));
    }
    payload.insert("presentation".into(), Value::String(presentation.clone()));
    let auth_token = option(&args, "--auth-token").or(saved_auth_token);
    if method == "search" {
        payload.insert(
            "mode".into(),
            Value::String(
                option(&args, "--mode").unwrap_or_else(|| die("--mode is required for search")),
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
    let save_path = option(&args, "--save-session-token")
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

fn option(args: &[String], name: &str) -> Option<String> {
    args.windows(2)
        .find(|pair| pair[0] == name)
        .map(|pair| pair[1].clone())
}

fn session_identity(args: &[String]) -> Option<String> {
    option(args, "--session")
        .or_else(|| option(args, "--agent"))
        .or_else(|| std::env::var("TSCH_AI_EDITOR_AGENT").ok())
        .or_else(|| std::env::var("CODEX_AGENT_ID").ok())
        .or_else(|| std::env::var("AGENT_ID").ok())
}

fn session_path(identity: &str) -> PathBuf {
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
    let key = blake3::hash(identity.as_bytes()).to_hex().to_string();
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
    println!("Usage: ai-text-editor COMMAND --endpoint ENDPOINT [OPTIONS]");
    println!("Commands: open history resources read insert replace large-edit begin-transaction end-transaction restore undo redo save save-as close resolve index cursor page search");
    println!(
        "         job-start job-poll job-progress job-complete job-cancel job-transfer job-release"
    );
    println!("Discovery: use --file PATH instead of --endpoint to read the announced endpoint; use --session-token PATH to reuse a saved endpoint and auth token.");
    println!(
        "Document modes: text_utf8, raw_bytes, hex_view (select at server start with --mode)."
    );
    println!("Search requires --mode and --query (or --query-base64): exact_text, exact_bytes, wildcard, shell_wildcard, path_wildcard, regex_rust, regex_pcre2, fuzzy_edit, fuzzy_subsequence, fuzzy_token, fuzzy_ngram, fuzzy_phonetic, fuzzy_soundex. Fuzzy modes accept --gradient 0.0..1.0 with strategy-specific defaults.");
    println!("Coordinates: text lines are 1-based and Unicode-scalar columns are 0-based; raw/hex coordinates are byte offsets. Refetch after every revision.");
    println!("Wrapped navigation: --wrap-width N adds visual coordinates; --visual interprets --line/--column as wrapped coordinates. Stored cursors remain logical.");
    println!("Edits: --offset N or --cursor-id N --delete-len N --text TEXT or --bytes-base64 B64; omitting --offset inserts/replaces at that cursor. --expected-revision N is required for safe concurrent edits. Use begin-transaction/end-transaction to group edits into one undo step.");
    println!("Reading: --before N --after N, --offset N --length N, --limit N, --pager-key KEY, --historical, --range-start-line N --range-end-line N, --range-start-byte N --range-end-byte N, --order forward|reverse.");
    println!("Presentation: --presentation structured|text|paging|stream; paging/stream readers must restart after the FILE EDITED delimiter.");
    println!("Recovery: resolve with --action backup|reload|merge|keep|force_save; backup preserves external bytes and leaves resolution pending, force-save requires --acknowledge-force-save. Add --preserve-external and optionally --backup-path PATH before discard/overwrite.");
    println!("Save-as: use save-as --target-path PATH to atomically create a new file without changing the active tab; existing targets are refused.");
    println!("Large edits: start a job, then use large-edit with --job-id N --acknowledge-large-edit --offset N --delete-len N and replacement data; this streams and atomically replaces the file.");
    println!("Close: close first prompts with journal_close_decision_required; repeat with --journal-action preserve|clean. clean deletes the tab journal and metadata database.");
    println!("Sessions/auth: --endpoint ENDPOINT, --session-token PATH, --session ID, --agent ID, or environment agent identity (TSCH_AI_EDITOR_AGENT, CODEX_AGENT_ID, AGENT_ID); explicit --endpoint wins, stale saved sessions are errors. --save-session-token PATH overrides automatic session storage. TCP clients use --auth-token TOKEN (or a token saved in the session file); servers accept --auth-token TOKEN or owner-only --auth-token-file PATH. Jobs use --job-id N, --owner NAME, --resume-token TOKEN, --detached, --progress-json JSON, --result-json JSON.");
    println!("The server alone owns document state, history, indexes, journals, and SQLite metadata. Large files provide bounded read/index views; acknowledged large-edit jobs stream accepted rewrites and retain file-backed undo snapshots.");
}
