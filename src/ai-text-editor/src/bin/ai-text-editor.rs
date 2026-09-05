// MODE: DEV
// PACKAGE: PROD
use ai_text_editor::client::{self, ResolveRequest};
use ai_text_editor::protocol::Envelope;
use ai_text_editor::transport::request;
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
    let explicit_identity =
        option(&args, &["--session", "-s"]).or_else(|| option(&args, &["--agent", "-A"]));
    let resolve_request = ResolveRequest {
        file: file.clone(),
        method: method.to_string(),
        explicit_endpoint: option(&args, &["--endpoint", "-e"]),
        explicit_identity,
        session_token_path: option(&args, &["--session-token"]).map(PathBuf::from),
        agent_env_var: "TSCH_AI_EDITOR_AGENT".to_string(),
        document_mode: option(&args, &["--document-mode", "-M"]),
        normalize_nfc: flag(&args, &["--normalize-nfc"]),
        idle_timeout_seconds: option(&args, &["--idle-timeout-seconds"]),
    };
    let resolved = client::resolve(&resolve_request).unwrap_or_else(|error| die(&error));
    let endpoint = resolved.endpoint;
    let saved_auth_token = resolved.auth_token;
    let saved_session_token = resolved.session_token;
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
        .or(resolved.cache_path);
    let returned_session_token = frames
        .iter()
        .find(|frame| frame.get("type").and_then(Value::as_str) == Some("data"))
        .and_then(|frame| frame.pointer("/payload/session_token"))
        .and_then(Value::as_str);
    let persisted_session_token = returned_session_token.or(saved_session_token.as_deref());
    // A refused request must not rewrite the per-(identity,file) cache:
    // persisting the endpoint and token that *answered* the refusal bakes a
    // wrong-file routing in, and every later command for this file replays
    // the same mismatch — a wedge that outlived the original mistake.
    if !failed {
        client::persist_cache(
            save_path.as_deref(),
            &endpoint,
            auth_token.as_deref(),
            persisted_session_token,
        )
        .unwrap_or_else(|error| die(&error));
    }
    // A refused operation must look refused in every presentation. Before
    // this, `text`/`paging`/`stream` dropped error frames silently, so a
    // stale revision or an unresolved external change exited 1 with empty
    // stdout and empty stderr — indistinguishable from a wedged server and
    // the exact failure that made batch edits unverifiable.
    if presentation != "structured" {
        for frame in &frames {
            if frame.get("type").and_then(Value::as_str) == Some("error") {
                let code = frame.get("code").and_then(Value::as_str).unwrap_or("error");
                let message = frame.get("message").and_then(Value::as_str).unwrap_or("");
                let details = frame
                    .get("details")
                    .map(|details| {
                        format!(" {}", serde_json::to_string(details).unwrap_or_default())
                    })
                    .unwrap_or_default();
                eprintln!("ai-text-editor: {code}: {message}{details}");
            }
        }
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

fn option(args: &[String], names: &[&str]) -> Option<String> {
    args.windows(2)
        .find(|pair| names.contains(&pair[0].as_str()))
        .map(|pair| pair[1].clone())
}

fn flag(args: &[String], names: &[&str]) -> bool {
    args.iter().any(|arg| names.contains(&arg.as_str()))
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
    println!("On Windows, where no usable Unix socket exists, the autostarted server falls back to loopback TCP on an ephemeral port with a per-start authentication token kept in a token file; every command works unchanged there.");
    println!("Opening a second file under the same agent identity (an explicit --session/--agent, or your coding harness's own session env vars) reconnects to that agent's already-running workspace and adds the file there as a new tab, rather than starting an unrelated second server.");
    println!("New files: open on a path that does not exist yet is not an error — the tab starts empty and the file is created on disk by the first successful save.");
    println!("Recovery: if the server died, open again (a stale endpoint whose owning process is gone is reclaimed automatically); reads report dirty/external_change_pending state, and every server refusal is named on stderr in every presentation.");
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
    println!("Edits: -o/--offset N (a BYTE offset into the document) or -C/--cursor-id N, plus -d/--delete-len N (bytes to delete from the offset; it may cross line ends and is reported back as spans_lines when it does) and -t/--text TEXT or --bytes-base64 B64; omitting -o inserts/replaces at that cursor. -r/--expected-revision N is required for safe concurrent edits. Edits are journal-and-buffer only: they return a new revision but nothing reaches the file until save succeeds; mutating responses carry a dirty flag. Use begin-transaction/end-transaction to group edits into one undo step.");
    println!("Reading: -b/--before N -B/--after N (line windows around the cursor), -o/--offset N -L/--length N (a BYTE window of the text, snapped to UTF-8 boundaries), -n/--limit N, --pager-key KEY, -H/--historical, --range-start-line N --range-end-line N, --range-start-byte N --range-end-byte N, --order forward|reverse.");
    println!("Presentation: -p/--presentation structured|text|paging|stream; paging/stream readers must restart after the FILE EDITED delimiter.");
    println!("Recovery: resolve with -a/--action backup|reload|merge|keep|force_save; backup preserves external bytes and leaves resolution pending, force-save requires --acknowledge-force-save. Add --preserve-external and optionally --backup-path PATH before discard/overwrite.");
    println!("Save-as: use save-as --target-path PATH to atomically create a new file without changing the active tab; existing targets are refused.");
    println!("Large edits: start a job, then use large-edit with -j/--job-id N --acknowledge-large-edit -o/--offset N -d/--delete-len N and replacement data; this streams and atomically replaces the file.");
    println!("Close: close first prompts with journal_close_decision_required; repeat with --journal-action preserve|clean. clean deletes the tab journal and metadata database.");
    println!("Sessions/auth: -e/--endpoint ENDPOINT, --session-token PATH, -s/--session ID, -A/--agent ID, or environment agent identity (TSCH_AI_EDITOR_AGENT, or a coding harness's own CLAUDE_CODE_SESSION_ID/CODEX_SESSION_ID/OPENCODE_PID); explicit --endpoint wins, stale saved sessions are errors. --save-session-token PATH overrides automatic session storage. TCP clients use --auth-token TOKEN (or a token saved in the session file); servers accept --auth-token TOKEN or owner-only --auth-token-file PATH. Jobs use -j/--job-id N, --owner NAME, --resume-token TOKEN, --detached, --progress-json JSON, --result-json JSON.");
    println!("The server alone owns document state, history, indexes, journals, and SQLite metadata. Large files provide bounded read/index views; acknowledged large-edit jobs stream accepted rewrites and retain file-backed undo snapshots.");
}
