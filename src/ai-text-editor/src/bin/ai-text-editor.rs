// MODE: DEV
// PACKAGE: PROD
use ai_text_editor::client::{self, ResolveRequest};
use ai_text_editor::protocol::Envelope;
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
        "jump-points" => "jump_points",
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
    let tab_id = option(&args, &["--tab-id", "-T"]);
    let tab_path = option(&args, &["--tab-path"]);
    let resolve_request = ResolveRequest {
        file: file.clone(),
        tab_id: tab_id.clone(),
        tab_path: tab_path.clone(),
        method: method.to_string(),
        explicit_endpoint: option(&args, &["--endpoint", "-e"]),
        explicit_identity,
        session_token_path: option(&args, &["--session-token"]).map(PathBuf::from),
        agent_env_var: "TSCH_AI_EDITOR_AGENT".to_string(),
        document_mode: option(&args, &["--document-mode", "-M"]),
        normalize_nfc: flag(&args, &["--normalize-nfc"]),
        idle_timeout_seconds: option(&args, &["--idle-timeout-seconds"]),
        acknowledge_create_parents: flag(&args, &["--acknowledge-create-parents"]),
        takeover_stale_endpoint: flag(&args, &["--takeover-stale-endpoint"]),
        force_refresh: false,
    };
    let mut payload = serde_json::Map::new();
    if let Some(file) = &file {
        payload.insert(
            "file".into(),
            Value::String(file.to_string_lossy().into_owned()),
        );
    }
    // T99: the response verbosity ladder. Every verb takes it, and the
    // server refuses a level outside 0..=3 by name rather than clamping.
    if let Some(value) = option(&args, &["--verbosity"]) {
        payload.insert(
            "verbosity".into(),
            json!(parse_number(&value, "--verbosity")),
        );
    }
    // T96/T97: the server routes on these, so they travel in the payload the
    // way `file` does rather than being consumed by the client alone.
    if let Some(value) = &tab_id {
        payload.insert("tab_id".into(), Value::String(value.clone()));
    }
    if let Some(value) = &tab_path {
        payload.insert("tab_path".into(), Value::String(value.clone()));
    }
    // B238: on `open`, -M/--document-mode is the mode of the TAB being
    // opened, so it travels in the payload as well as into the autostart
    // argv. On every other verb it shapes only a server this call starts,
    // and the server refuses it as an argument that verb does not read.
    if method == "open" {
        if let Some(value) = option(&args, &["--document-mode", "-M"]) {
            payload.insert("document_mode".into(), Value::String(value));
        }
    }
    if let Some(value) = option(&args, &["--bytes-base64"]) {
        payload.insert("bytes_base64".into(), Value::String(value));
    }
    if let Some(value) = option(&args, &["--id"]) {
        payload.insert("id".into(), json!(parse_number(&value, "--id")));
    }
    if let Some(value) = option(&args, &["--line", "-l"]) {
        payload.insert("line".into(), json!(parse_number(&value, "--line")));
    }
    if let Some(value) = option(&args, &["--column", "-c"]) {
        payload.insert("column".into(), json!(parse_number(&value, "--column")));
    }
    if let Some(value) = option(&args, &["--action", "-a"]) {
        payload.insert("action".into(), Value::String(value));
    }
    if let Some(value) = option(&args, &["--page-lines"]) {
        payload.insert(
            "page_lines".into(),
            json!(parse_number(&value, "--page-lines")),
        );
    }
    if let Some(value) = option(&args, &["--wrap-width", "-w"]) {
        payload.insert(
            "wrap_width".into(),
            json!(parse_number(&value, "--wrap-width")),
        );
    }
    if flag(&args, &["--visual", "-V"]) {
        payload.insert("visual".into(), Value::Bool(true));
    }
    for (names, field) in [(["--before", "-b"], "before"), (["--after", "-B"], "after")] {
        if let Some(value) = option(&args, &names) {
            payload.insert(field.into(), json!(parse_number(&value, names[0])));
        }
    }
    for (argument, field) in [
        ("--range-start-line", "range_start_line"),
        ("--range-end-line", "range_end_line"),
        ("--range-start-byte", "range_start_byte"),
        ("--range-end-byte", "range_end_byte"),
    ] {
        if let Some(value) = option(&args, &[argument]) {
            payload.insert(field.into(), json!(parse_number(&value, argument)));
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
    let expected_revision = option(&args, &["--expected-revision", "-r"])
        .map(|value| parse_number(&value, "--expected-revision"));
    if let Some(value) = option(&args, &["--offset", "-o"]) {
        payload.insert("offset".into(), json!(parse_number(&value, "--offset")));
    }
    if let Some(value) = option(&args, &["--length", "-L"]) {
        payload.insert("length".into(), json!(parse_number(&value, "--length")));
    }
    if let Some(value) = option(&args, &["--granularity"]) {
        payload.insert(
            "granularity".into(),
            json!(parse_number(&value, "--granularity")),
        );
    }
    if let Some(value) = option(&args, &["--pager-key"]) {
        payload.insert("pager_key".into(), Value::String(value));
    }
    if let Some(value) = option(&args, &["--limit", "-n"]) {
        payload.insert("limit".into(), json!(parse_number(&value, "--limit")));
    }
    if let Some(value) = option(&args, &["--delete-len", "-d"]) {
        payload.insert(
            "delete_len".into(),
            json!(parse_number(&value, "--delete-len")),
        );
    }
    for (argument, field) in [
        ("--expected-text", "expected_text"),
        ("--expected-bytes-base64", "expected_bytes_base64"),
        ("--match-id", "match_id"),
    ] {
        if let Some(value) = option(&args, &[argument]) {
            payload.insert(field.into(), Value::String(value));
        }
    }
    if let Some(value) = option(&args, &["--preview-lines"]) {
        payload.insert(
            "preview_lines".into(),
            json!(parse_number(&value, "--preview-lines")),
        );
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
                payload.insert(field.into(), json!(parse_number(&value, argument)));
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
    let auth_token = option(&args, &["--auth-token"]);
    if method == "search" {
        payload.insert(
            "mode".into(),
            Value::String(
                option(&args, &["--mode", "-m"])
                    .unwrap_or_else(|| die("--mode is required for search")),
            ),
        );
    }
    let method = method.to_string();
    let payload = Value::Object(payload);
    // Bound here so the rejection below sees it as consulted (B207), and the
    // persistence decision below still runs after the response.
    let saved_token_path = option(&args, &["--save-session-token"]).map(PathBuf::from);
    // B207: refuse unknown options before anything leaves the process; a
    // server-side unknown_argument guard can only protect keys that arrive.
    reject_unknown_options(&args);
    let (frames, resolved) = client::execute(&resolve_request, |resolved| Envelope {
        version: ai_text_editor::PROTOCOL_VERSION,
        request_id: "cli-1".into(),
        method: method.clone(),
        revision: expected_revision,
        auth_token: auth_token.clone().or_else(|| resolved.auth_token.clone()),
        session_token: resolved.session_token.clone(),
        payload: payload.clone(),
    })
    // B213: a valid command line whose workspace disappeared underneath it
    // (the documented idle-reap) is not a usage error; sysexits-style 66 says
    // "no such thing" and leaves 64 for genuine argument mistakes.
    .unwrap_or_else(|error| die_runtime(&error));
    let failed = frames
        .iter()
        .any(|frame| frame.get("type").and_then(Value::as_str) == Some("error"));
    let save_path = saved_token_path.or(resolved.cache_path);
    let returned_session_token = frames
        .iter()
        .find(|frame| frame.get("type").and_then(Value::as_str) == Some("data"))
        .and_then(|frame| frame.pointer("/payload/session_token"))
        .and_then(Value::as_str);
    let persisted_session_token = returned_session_token.or(resolved.session_token.as_deref());
    // A refused request must not rewrite the per-(identity,file) cache:
    // persisting the endpoint and token that *answered* the refusal bakes a
    // wrong-file routing in, and every later command for this file replays
    // the same mismatch — a wedge that outlived the original mistake.
    if !failed {
        client::persist_cache(
            save_path.as_deref(),
            &resolved.endpoint,
            auth_token.as_deref().or(resolved.auth_token.as_deref()),
            persisted_session_token,
        )
        .unwrap_or_else(|error| die(&error));
        // T98: a successful call focuses the tab that served it, so the next
        // request naming nothing is served by the same tab. Not on a refusal:
        // the tab that answered one is not the tab the caller meant.
        client::persist_focus(
            &resolve_request,
            &resolved.endpoint,
            auth_token.as_deref().or(resolved.auth_token.as_deref()),
            persisted_session_token,
        );
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

thread_local! {
    // B207: the pull-based parser below reads only the names it asks for, so
    // an unknown option used to vanish without a trace and the operation
    // succeeded with whatever the dropped argument would have changed. Each
    // lookup registers the names it consulted; `reject_unknown_options` then
    // requires every option-shaped token in the command line to have been
    // consulted, or to sit in the value position after a name that takes one.
    static VALUE_OPTIONS: std::cell::RefCell<Vec<String>> =
        const { std::cell::RefCell::new(Vec::new()) };
    static BOOL_OPTIONS: std::cell::RefCell<Vec<String>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

fn option(args: &[String], names: &[&str]) -> Option<String> {
    VALUE_OPTIONS.with(|known| {
        let mut known = known.borrow_mut();
        known.extend(names.iter().map(|name| (*name).to_string()));
    });
    args.windows(2)
        .find(|pair| names.contains(&pair[0].as_str()))
        .map(|pair| pair[1].clone())
}

fn flag(args: &[String], names: &[&str]) -> bool {
    BOOL_OPTIONS.with(|known| {
        let mut known = known.borrow_mut();
        known.extend(names.iter().map(|name| (*name).to_string()));
    });
    args.iter().any(|arg| names.contains(&arg.as_str()))
}

fn reject_unknown_options(args: &[String]) {
    let unknown = args.iter().enumerate().find(|(index, token)| {
        if *index < 2 || !token.starts_with('-') || *token == "-" {
            return false;
        }
        let known = VALUE_OPTIONS.with(|names| names.borrow().contains(token))
            || BOOL_OPTIONS.with(|names| names.borrow().contains(token));
        if known {
            return false;
        }
        let value_position = args
            .get(index - 1)
            .is_some_and(|previous| VALUE_OPTIONS.with(|names| names.borrow().contains(previous)));
        !value_position
    });
    if let Some((_, token)) = unknown {
        die(&format!(
            "unknown option {token} for command {}; every option a command reads is listed in `ai-text-editor help`",
            args.get(1).map(String::as_str).unwrap_or("")
        ));
    }
}

fn parse_number(value: &str, flag: &str) -> u64 {
    value.parse().unwrap_or_else(|_| {
        // B202: name the argument at fault, not the token it swallowed —
        // an empty shell variable left `-r` holding the next flag, and the
        // old text blamed that flag.
        die(&format!(
            "{flag} requires a non-negative integer, got {value:?}"
        ))
    })
}
fn die(message: &str) -> ! {
    eprintln!("ai-text-editor: {message}");
    std::process::exit(64);
}
fn die_runtime(message: &str) -> ! {
    // B213: distinct from usage; the command line was fine, the world moved.
    eprintln!("ai-text-editor: {message}");
    std::process::exit(66);
}
fn help() {
    println!("Usage: ai-text-editor COMMAND -f FILE [OPTIONS]  (or --endpoint/-e ENDPOINT for an already-open tab)");
    println!("open starts its own server when none is running yet: no separate `ai-text-editor-server start` call is needed. Use --document-mode/-M and --normalize-nfc to shape that autostart; open --endpoint ENDPOINT -f PATH adds another isolated tab to a server that is already up.");
    println!("On Windows, where no usable Unix socket exists, the autostarted server falls back to loopback TCP on an ephemeral port with a per-start authentication token kept in a token file; every command works unchanged there.");
    println!("Opening a second file under the same agent identity (an explicit --session/--agent, or your coding harness's own session env vars) reconnects to that agent's already-running workspace and adds the file there as a new tab, rather than starting an unrelated second server.");
    println!("New files: open on a path that does not exist yet is not an error — the tab starts empty and the file is created on disk by the first successful save.");
    println!("Recovery: if the server died, open again (a stale endpoint whose owning process is gone is reclaimed automatically); reads report dirty/external_change_pending state, and every server refusal is named on stderr in every presentation.");
    println!("Stale endpoints: when the recorded owner is still alive or cannot be ruled out, the start is refused with that pid and generation named. Verify the process is gone, then repeat the command with --takeover-stale-endpoint; the old record is kept under a stale- suffix rather than overwritten.");
    println!("Commands: open capabilities history jump-points resources read insert replace large-edit begin-transaction end-transaction restore undo redo save save-as close resolve index cursor page search");
    println!(
        "         job-start job-poll job-progress job-complete job-cancel job-transfer job-release"
    );
    println!("Response size: --verbosity 0|1|2|3 (default 1). 0 is the answer and the tab that gave it, nothing else. 1 adds what verification and the next step need - revision, dirty, the tab mode, the resolved edit span, completeness, and a searchs pager key and count. 2 adds navigation - cursors, byte windows, block paging, undo depths. 3 is everything. capabilities and resources are exempt (their payload IS metadata), and a refusal always carries its full code, message and choices whatever the level.");
    println!("Addressing a tab: every response reports a tab_id, and -T/--tab-id ID addresses that tab for any command with no --file and no --endpoint. --tab-path FRAGMENT names a tab by filename or by a trailing run of path components (component boundaries, not substrings) and is the recovery when the id is lost; several matches are refused with tab_ambiguous AND the candidates with their ids, none with tab_unmatched and the open tabs. A command naming nothing runs on the focused tab - the tab the last successful call was served by, which open sets and a refusal never moves.");
    println!("Common flags (long / short): --file -f, --tab-id -T, --tab-path, --endpoint -e, --line -l, --column -c, --action -a, --text -t, --query -q, --mode -m (search only), --expected-revision -r, --offset -o, --length -L, --delete-len -d, --limit -n, --cursor-id -C (edit anchor), --id (which numbered cursor a navigation command moves; default 0), --job-id -j, --presentation -p, --before -b, --after -B, --gradient -g, --wrap-width -w, --session -s, --agent -A.");
    println!("Navigation: --id N routes a cursor command to numbered cursor N (every cursor is created by its first move); home and end move to the first and last column of the CURRENT line, not the document's start or end; next_word/previous_word step words; page_up/page_down move --page-lines N lines (default 40).");
    println!("Exit codes: 0 success; 64 usage, including any option the command does not read; 66 no such tab, file, or reachable server (run open first); 1 when the server itself refused, with the refusal code printed.");
    println!("Boolean flags with a short form: --visual -V, --historical -H. Safety acknowledgements (--acknowledge-force-save, --acknowledge-large-edit) and auth/session flags are deliberately long-form only.");
    println!("Document modes: text_utf8, raw_bytes, hex_view. On open, -M/--document-mode is the mode of the TAB being opened, whether or not a workspace is already running; a tab's mode is fixed for its lifetime, so reopening one under a different mode is refused with document_mode_conflict and close+open is how to change it. --mode selects it when starting a server yourself.");
    println!("Search requires -m/--mode and -q/--query (or --query-base64): exact_text, exact_bytes, wildcard, shell_wildcard, path_wildcard, regex_rust, regex_pcre2, fuzzy_edit, fuzzy_subsequence, fuzzy_token, fuzzy_ngram, fuzzy_phonetic, fuzzy_soundex. Fuzzy modes accept -g/--gradient 0.0..1.0 with strategy-specific defaults. exact_bytes decodes its query as base64-encoded bytes; plain text belongs in exact_text.");
    println!("Coordinates: text lines are 1-based and Unicode-scalar columns are 0-based; raw/hex coordinates are byte offsets. Refetch after every revision.");
    println!("Wrapped navigation: -w/--wrap-width N adds visual coordinates; -V/--visual interprets -l/-c as wrapped coordinates. Stored cursors remain logical.");
    println!("Edits: -o/--offset N (a BYTE offset into the document) or -C/--cursor-id N, plus -d/--delete-len N (bytes to delete from the offset; it may cross line ends and is reported back as spans_lines when it does) and -t/--text TEXT or --bytes-base64 B64; omitting -o inserts/replaces at that cursor. For replace, a whole span can be addressed directly instead of by arithmetic: --range-start-line N --range-end-line N (inclusive, 1-based, the last line's newline included, so replacing with no text deletes the lines outright) or --range-start-byte N --range-end-byte N (half-open, exactly what a search hit reports as byte_start/byte_end, so a span across two hits is those two numbers copied across). A range may not be combined with -o, -d or -C, and insert takes no range - it places bytes at a point. -r/--expected-revision N is required for safe concurrent edits. Edits are journal-and-buffer only: they return a new revision but nothing reaches the file until save succeeds; mutating responses carry a dirty flag. Use begin-transaction/end-transaction to group edits into one undo step.");
    println!("Reading: -b/--before N -B/--after N (line window around the cursor), -o/--offset N -L/--length N (a BYTE window of the text, snapped to UTF-8 boundaries), --range-start-line N --range-end-line N (an inclusive line window on text tabs), --range-start-byte N --range-end-byte N (a half-open byte window on raw and hex tabs).");
    println!("Paging search results: -n/--limit N, --pager-key KEY, --historical, and the page command's -o/--offset N; --order forward|reverse applies to search responses. A search command itself refuses -o/--offset: the offset pages an existing result set, it never trims a fresh scan.");
    println!("Presentation: -p/--presentation structured|text|paging|stream; paging/stream readers must restart after the FILE EDITED delimiter.");
    println!("Recovery: resolve with -a/--action backup|reload|merge|keep|force_save; backup preserves external bytes and leaves resolution pending, force-save requires --acknowledge-force-save. Add --preserve-external and optionally --backup-path PATH before discard/overwrite.");
    println!("Save-as: use save-as --target-path PATH to atomically create a new file without changing the active tab; existing targets are refused.");
    println!("Large edits: start a job, then use large-edit with -j/--job-id N --resume-token TOKEN --acknowledge-large-edit -o/--offset N -d/--delete-len N, -r/--expected-revision N and replacement data; this streams and atomically replaces the file. Job verbs (poll/progress/complete/cancel/transfer/release) all require --resume-token; it is issued by job-start and never disclosed to a caller who does not hold it.");
    println!("Close: close first prompts with journal_close_decision_required; repeat with --journal-action preserve|clean. clean deletes the tab journal and metadata database.");
    println!("Sessions/auth: -e/--endpoint ENDPOINT, --session-token PATH, -s/--session ID, -A/--agent ID, or environment agent identity (TSCH_AI_EDITOR_AGENT, or a coding harness's own CLAUDE_CODE_SESSION_ID/CODEX_SESSION_ID/OPENCODE_PID); explicit --endpoint wins, stale saved sessions are errors. --save-session-token PATH overrides automatic session storage. TCP clients use --auth-token TOKEN (or a token saved in the session file); servers accept --auth-token TOKEN or owner-only --auth-token-file PATH. Jobs use -j/--job-id N, --owner NAME, --resume-token TOKEN, --detached, --progress-json JSON, --result-json JSON.");
    println!("The server alone owns document state, history, indexes, journals, and SQLite metadata. Large files provide bounded read/index views; acknowledged large-edit jobs stream accepted rewrites and retain file-backed undo snapshots.");
}
