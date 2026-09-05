// MODE: DEV
// PACKAGE: PROD
//! Optional MCP adapter for the editor protocol.

use ai_text_editor::client::{self, ResolveRequest};
use ai_text_editor::protocol::Envelope;
use serde_json::{json, Value};
use std::path::PathBuf;

pub fn handle(message: Value) -> Value {
    let id = message.get("id").cloned().unwrap_or(Value::Null);
    let method = message.get("method").and_then(Value::as_str).unwrap_or("");
    match method {
        "initialize" => {
            json!({"jsonrpc":"2.0","id":id,"result":{"protocolVersion":"2025-06-18","capabilities":{"tools":{},"resources":{"subscribe":false,"listChanged":false}},"serverInfo":{"name":"ai-text-editor","version":"0.1.0"}}})
        }
        "notifications/initialized" => Value::Null,
        "tools/list" => json!({"jsonrpc":"2.0","id":id,"result":{"tools": tool_definitions()}}),
        "resources/list" => {
            json!({"jsonrpc":"2.0","id":id,"result":{"resources": resource_definitions()}})
        }
        "resources/read" => read_resource(id, message.get("params").cloned().unwrap_or_default()),
        "tools/call" => call_tool(id, message.get("params").cloned().unwrap_or_default()),
        _ => json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"method not found"}}),
    }
}

fn resource_definitions() -> Vec<Value> {
    [
        ("ai-text-editor://schemas/protocol.v1.json", "Versioned NDJSON request and response schema.", "application/json"),
        ("ai-text-editor://schemas/capabilities.v1.json", "Advertised document, search, presentation, and job capabilities.", "application/json"),
        ("ai-text-editor://ai-text-editor.1", "Complete command and responsibility reference.", "text/plain"),
    ]
    .into_iter()
    .map(|(uri, description, mime_type)| json!({"uri":uri,"name":uri,"description":description,"mimeType":mime_type}))
    .collect()
}

fn read_resource(id: Value, params: Value) -> Value {
    let uri = params.get("uri").and_then(Value::as_str).unwrap_or("");
    let (text, mime_type) = match uri {
        "ai-text-editor://schemas/protocol.v1.json" => (
            include_str!("../../../ai-text-editor/schemas/protocol.v1.json"),
            "application/json",
        ),
        "ai-text-editor://schemas/capabilities.v1.json" => (
            include_str!("../../../ai-text-editor/schemas/capabilities.v1.json"),
            "application/json",
        ),
        "ai-text-editor://ai-text-editor.1" => (
            include_str!("../../../ai-text-editor/ai-text-editor.1"),
            "text/plain",
        ),
        _ => {
            return json!({"jsonrpc":"2.0","id":id,"error":{"code":-32002,"message":"unknown resource URI"}})
        }
    };
    json!({"jsonrpc":"2.0","id":id,"result":{"contents":[{"uri":uri,"mimeType":mime_type,"text":text}]}})
}

fn int(description: &str) -> Value {
    json!({"type": "integer", "description": description})
}

fn string(description: &str) -> Value {
    json!({"type": "string", "description": description})
}

fn boolean(description: &str) -> Value {
    json!({"type": "boolean", "description": description})
}

fn revision_guard() -> Value {
    string("Server revision guard; REQUIRED for this operation and must be the revision most recently returned by open, history, or a completed mutation. Missing revisions are refused, stale ones never merged.")
}

fn number(description: &str) -> Value {
    json!({"type": "number", "description": description})
}

fn mutating_required() -> Vec<&'static str> {
    vec!["expected_revision"]
}

type ToolProperties = Vec<(&'static str, Value)>;
type ToolSpec = (
    &'static str,
    &'static str,
    ToolProperties,
    Vec<&'static str>,
);

fn tool_definitions() -> Vec<Value> {
    let routing = || {
        Vec::from([
            ("file", string("Path served by this request; routes to that file's own tab in the agent's workspace.")),
            ("endpoint", string("Explicit editor endpoint (unix:/path or host:port); wins over discovery.")),
            ("agent", string("Agent identity used to reconnect to this agent's running workspace.")),
            ("session", string("Session identity; same resolution as agent.")),
        ])
    };
    let mut tools: Vec<ToolSpec> = Vec::new();
    tools.push(("open", "Inspect the tab path, document mode, revision, size, and cursors. Opens the file if the workspace does not have it yet; starting a server when none runs. document_mode and normalize_nfc only shape an autostarted server - when a workspace already runs, the tab reports what it actually is.", routing(), vec![]));
    tools.push(("capabilities", "Inspect the machine-readable protocol modes, coordinate rules, defaults, resource limits, and transports. Answers from the running server when one is reachable, from compiled-in defaults (marked source: client_default) otherwise.", routing(), vec![]));
    tools.push(("resources", "Inspect available memory, server overhead, working-set recommendation, and large-file threshold.", routing(), vec![]));
    tools.push((
        "history",
        "Inspect undo/redo depths and journal sequence without changing the tab.",
        routing(),
        vec![],
    ));
    tools.push(("read", "Read the current document or a bounded byte/line range. Line ranges are inclusive on text tabs; byte ranges half-open on raw and hex tabs; offset/length is a byte window snapped to UTF-8 boundaries.", { let mut p = routing(); p.extend(Vec::from([
        ("cursor_id", int("Numeric cursor whose position anchors a before/after window.")),
        ("before", int("Lines before the cursor to include.")),
        ("after", int("Lines after the cursor to include.")),
        ("line", int("Large text tabs: a specific 1-based line to read.")),
        ("offset", int("Byte offset of a bounded window.")),
        ("length", int("Byte length of a bounded window.")),
        ("range_start_line", int("Inclusive first line of a line-range read (text tabs).")),
        ("range_end_line", int("Inclusive last line of a line-range read (text tabs).")),
        ("range_start_byte", int("Inclusive first byte of a byte-range read (raw and hex tabs).")),
        ("range_end_byte", int("Exclusive last byte of a byte-range read (raw and hex tabs).")),
    ])); p }, vec![]));
    tools.push((
        "insert",
        "Insert text or base64 bytes at a byte offset; preserve the revision guard.",
        {
            let mut p = routing();
            p.extend(Vec::from([
                (
                    "offset",
                    int("Byte offset; omitted means the cursor position."),
                ),
                (
                    "cursor_id",
                    int("Numeric cursor to insert at when offset is omitted."),
                ),
                ("text", string("Text to insert.")),
                (
                    "bytes_base64",
                    string("Base64 bytes to insert, alternative to text."),
                ),
                ("expected_revision", revision_guard()),
            ]));
            p
        },
        mutating_required(),
    ));
    tools.push((
        "replace",
        "Replace a byte range with text or base64 bytes; preserve the revision guard.",
        {
            let mut p = routing();
            p.extend(Vec::from([
                (
                    "offset",
                    int("Byte offset; omitted means the cursor position."),
                ),
                (
                    "cursor_id",
                    int("Numeric cursor to replace at when offset is omitted."),
                ),
                ("delete_len", int("Byte length to delete before inserting.")),
                ("text", string("Replacement text.")),
                (
                    "bytes_base64",
                    string("Replacement bytes, alternative to text."),
                ),
                ("expected_revision", revision_guard()),
            ]));
            p
        },
        mutating_required(),
    ));
    tools.push(("large_edit", "Stream an acknowledged job-owned rewrite of a large file and atomically replace it.", { let mut p = routing(); p.extend(Vec::from([
        ("job_id", int("Queued job this edit executes.")),
        ("resume_token", string("The job's resume token; required, and never disclosed to callers without it.")),
        ("acknowledge_large_edit", boolean("Must be true; confirms the streamed rewrite cost.")),
        ("offset", int("Byte offset of the rewrite.")),
        ("delete_len", int("Byte length replaced.")),
        ("text", string("Replacement text.")),
        ("bytes_base64", string("Replacement bytes, alternative to text.")),
        ("expected_revision", revision_guard()),
    ])); p }, mutating_required()));
    tools.push(("begin_transaction", "Begin an explicit undo transaction; subsequent ordinary edits are grouped until end_transaction.", { let mut p = routing(); p.extend(Vec::from([("expected_revision", revision_guard())])); p }, mutating_required()));
    tools.push((
        "end_transaction",
        "Close the explicit undo transaction and commit its grouped undo step.",
        {
            let mut p = routing();
            p.extend(Vec::from([("expected_revision", revision_guard())]));
            p
        },
        mutating_required(),
    ));
    tools.push(("restore", "Turn off lossless NFC presentation; refuses with not_normalized when the tab never normalized, and refuses when edits made restoration lossy.", { let mut p = routing(); p.extend(Vec::from([("expected_revision", revision_guard())])); p }, mutating_required()));
    tools.push((
        "undo",
        "Undo one server history transaction; the server returns the new revision.",
        {
            let mut p = routing();
            p.extend(Vec::from([("expected_revision", revision_guard())]));
            p
        },
        mutating_required(),
    ));
    tools.push((
        "redo",
        "Redo one server history transaction; the server returns the new revision.",
        {
            let mut p = routing();
            p.extend(Vec::from([("expected_revision", revision_guard())]));
            p
        },
        mutating_required(),
    ));
    tools.push((
        "save",
        "Atomically save the current working view after external-change resolution.",
        {
            let mut p = routing();
            p.extend(Vec::from([("expected_revision", revision_guard())]));
            p
        },
        mutating_required(),
    ));
    tools.push(("save_as", "Atomically create a new target file without changing the active tab; existing targets are refused.", { let mut p = routing(); p.extend(Vec::from([("target_path", string("New file to create; must not already exist."))])); p }, vec![]));
    tools.push((
        "close",
        "Close the tab; first obtain and then explicitly choose journal preservation or cleanup.",
        {
            let mut p = routing();
            p.extend(Vec::from([(
                "journal_action",
                string("preserve or clean; clean deletes the tab journal and metadata database."),
            )]));
            p
        },
        vec![],
    ));
    tools.push((
        "resolve",
        "Resolve an external change with backup, reload, merge, keep, or acknowledged force_save.",
        {
            let mut p = routing();
            p.extend(Vec::from([
                (
                    "action",
                    string("backup, reload, merge, keep, or force_save."),
                ),
                (
                    "backup_path",
                    string("Where backup writes the preserved external copy."),
                ),
                (
                    "preserve_external",
                    boolean("Copy external bytes to a .back file before discard or overwrite."),
                ),
                (
                    "acknowledge_force_save",
                    boolean("Required for action force_save."),
                ),
            ]));
            p
        },
        vec![],
    ));
    tools.push((
        "index",
        "Build or inspect the lazy line/byte index, optionally with explicit granularity.",
        {
            let mut p = routing();
            p.extend(Vec::from([
                ("action", string("build or inspect.")),
                ("granularity", int("Lines per index block.")),
                ("offset", int("Block offset when paging index blocks.")),
                ("limit", int("Blocks to return.")),
            ]));
            p
        },
        vec![],
    ));
    tools.push((
        "cursor",
        "Create, move, or inspect numeric cursors and navigation positions.",
        {
            let mut p = routing();
            p.extend(Vec::from([
                (
                    "action",
                    string(
                        "home, end, next_word, previous_word, page_up, page_down, line, or column.",
                    ),
                ),
                ("id", int("Numeric cursor id.")),
                ("line", int("1-based destination line.")),
                ("column", int("0-based Unicode-scalar destination column.")),
                ("page_lines", int("Lines paged by page_up/page_down.")),
                (
                    "wrap_width",
                    int("Visual row width for wrapped coordinates."),
                ),
                (
                    "visual",
                    boolean("Interpret line/column as wrapped visual coordinates."),
                ),
            ]));
            p
        },
        vec![],
    ));
    tools.push(("page", "Page a previous search or index result set by its pager key. Pages are refetched after any write; a stale generation is refused by name.", { let mut p = routing(); p.extend(Vec::from([
        ("pager_key", string("Pager key from a previous search or index response.")),
        ("offset", int("Zero-based match offset to resume from.")),
        ("limit", int("Matches to return.")),
    ])); p }, vec![]));
    tools.push(("search", "Run exactly one explicit search mode and receive an immutable result id and pager key. Offset is not a search argument: page the result with `page`.", { let mut p = routing(); p.extend(Vec::from([
        ("mode", string("exact_text, exact_bytes, wildcard, shell_wildcard, path_wildcard, regex_rust, regex_pcre2, fuzzy_edit, fuzzy_subsequence, fuzzy_token, fuzzy_ngram, fuzzy_phonetic, fuzzy_soundex.")),
        ("query", string("Search query, interpreted by mode.")),
        ("query_base64", string("Base64 query, alternative to query.")),
        ("limit", int("Preview matches to return (default 4).")),
        ("pager_key", string("Existing result set to re-page instead of rescanning.")),
        ("historical", boolean("Replay the stored result set for this query rather than rescanning.")),
        ("order", string("forward or reverse.")),
        ("gradient", number("Fuzzy score floor between 0.0 and 1.0.")),
        ("range_start_line", int("Inclusive first line (required on large tabs).")),
        ("range_end_line", int("Inclusive last line (required on large tabs).")),
        ("range_start_byte", int("Inclusive first byte for exact_bytes on large tabs.")),
        ("range_end_byte", int("Exclusive last byte for exact_bytes on large tabs.")),
    ])); p }, vec!["mode", "query"]));
    tools.push((
        "job_start",
        "Create a lifecycle record for agent-owned long work; this tool does not execute the work.",
        {
            let mut p = routing();
            p.extend(Vec::from([
                ("owner", string("Name of the driving agent or process.")),
                (
                    "detached",
                    boolean("Keep the job active without a client connection."),
                ),
            ]));
            p
        },
        vec![],
    ));
    tools.push(("job_poll", "Read the current state, progress, and result of a job. Requires the resume_token issued at start; the token is never disclosed to a caller without it.", routing().into_iter().chain(Vec::from([("job_id", int("Job id.")), ("resume_token", string("Token from job_start; required."))])).collect(), vec!["job_id", "resume_token"]));
    tools.push((
        "job_progress",
        "Publish truthful progress for work the driving agent owns.",
        {
            let mut p = routing();
            p.extend(Vec::from([
                ("job_id", int("Job id.")),
                ("resume_token", string("Token from job_start; required.")),
                (
                    "progress",
                    json!({"description": "Free-form progress object."}),
                ),
            ]));
            p
        },
        vec!["job_id", "resume_token"],
    ));
    tools.push(("job_complete", "Publish a terminal result for work the driving agent owns. The result must be a JSON array of result frames; a single object is accepted and wrapped, never dropped.", { let mut p = routing(); p.extend(Vec::from([
        ("job_id", int("Job id.")),
        ("resume_token", string("Token from job_start; required.")),
        ("result", json!({"description": "Array of result frames (a bare object is wrapped into one)."})),
    ])); p }, vec!["job_id", "resume_token"]));
    tools.push((
        "job_cancel",
        "Cancel a non-terminal job; cancellation wins races with completion.",
        routing()
            .into_iter()
            .chain(Vec::from([
                ("job_id", int("Job id.")),
                ("resume_token", string("Token from job_start; required.")),
            ]))
            .collect(),
        vec!["job_id", "resume_token"],
    ));
    tools.push((
        "job_transfer",
        "Transfer ownership using the current resume token.",
        {
            let mut p = routing();
            p.extend(Vec::from([
                ("job_id", int("Job id.")),
                ("resume_token", string("Current token; required.")),
                ("owner", string("New owner name.")),
            ]));
            p
        },
        vec!["job_id", "resume_token", "owner"],
    ));
    tools.push((
        "job_release",
        "Permanently release a job and invalidate its resume token.",
        routing()
            .into_iter()
            .chain(Vec::from([
                ("job_id", int("Job id.")),
                ("resume_token", string("Current token; required.")),
            ]))
            .collect(),
        vec!["job_id", "resume_token"],
    ));
    tools
        .into_iter()
        .map(|(name, description, props, required)| {
            let mut properties = serde_json::Map::new();
            for (key, value) in routing() {
                properties.insert(key.to_string(), value);
            }
            for (key, value) in props {
                properties.insert(key.to_string(), value);
            }
            json!({
                "name": name,
                "description": format!("{description} Either an endpoint, or a file (for open) plus an optional agent/session id to reconnect to that agent's already-running workspace, resolves the target; a new workspace-server is started automatically if `open` finds none. The server is authoritative and every mutation needs a current revision."),
                "inputSchema": {
                    "type": "object",
                    "properties": Value::Object(properties),
                    "required": required,
                    "additionalProperties": false
                }
            })
        })
        .collect()
}

fn call_tool(id: Value, params: Value) -> Value {
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let method = if name == "resolve" {
        "resolve_external"
    } else {
        name
    };
    let mut payload = arguments.as_object().cloned().unwrap_or_default();
    let file = payload
        .get("file")
        .and_then(Value::as_str)
        .map(PathBuf::from);
    let resolve_request = ResolveRequest {
        file: file.clone(),
        method: method.to_string(),
        explicit_endpoint: payload
            .get("endpoint")
            .and_then(Value::as_str)
            .map(str::to_owned),
        explicit_identity: payload
            .get("session")
            .or_else(|| payload.get("agent"))
            .and_then(Value::as_str)
            .map(str::to_owned),
        session_token_path: None, // no equivalent of the CLI's --session-token file over MCP
        agent_env_var: "TSCH_AI_EDITOR_AGENT".to_string(),
        document_mode: payload
            .get("document_mode")
            .and_then(Value::as_str)
            .map(str::to_owned),
        normalize_nfc: payload
            .get("normalize_nfc")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        idle_timeout_seconds: payload
            .get("idle_timeout_seconds")
            .and_then(Value::as_u64)
            .map(|value| value.to_string()),
        force_refresh: false,
    };
    payload.remove("endpoint");
    payload.remove("session");
    payload.remove("agent");
    payload.remove("document_mode");
    payload.remove("normalize_nfc");
    payload.remove("idle_timeout_seconds");
    // B175: accept both the envelope's wire name (`revision`) and the
    // documented `expected_revision`, so a client that follows the schema can
    // actually satisfy the server's revision guard.
    let revision = payload
        .remove("expected_revision")
        .or_else(|| payload.remove("revision"))
        .and_then(|value| value.as_u64());
    let auth_token = payload
        .remove("auth_token")
        .and_then(|value| value.as_str().map(str::to_owned));
    let session_token = payload
        .remove("session_token")
        .and_then(|value| value.as_str().map(str::to_owned));
    let payload = Value::Object(payload);
    let method = method.to_string();
    let (frames, resolved) = match client::execute(&resolve_request, |resolved| Envelope {
        version: ai_text_editor::PROTOCOL_VERSION,
        request_id: "mcp-1".into(),
        method: method.clone(),
        revision,
        auth_token: auth_token.clone().or_else(|| resolved.auth_token.clone()),
        session_token: session_token
            .clone()
            .or_else(|| resolved.session_token.clone()),
        payload: payload.clone(),
    }) {
        Ok(pair) => pair,
        Err(error) => return tool_error(id, &error),
    };
    let failed = frames
        .iter()
        .any(|frame| frame.get("type").and_then(Value::as_str) == Some("error"));
    let returned_session_token = frames
        .iter()
        .find(|frame| frame.get("type").and_then(Value::as_str) == Some("data"))
        .and_then(|frame| frame.pointer("/payload/session_token"))
        .and_then(Value::as_str);
    // Same rule as the CLI: a refused request must not rewrite the
    // per-(identity,file) cache with the endpoint and token of
    // whichever tab answered the refusal.
    if !failed {
        let _ = client::persist_cache(
            resolved.cache_path.as_deref(),
            &resolved.endpoint,
            auth_token.as_deref().or(resolved.auth_token.as_deref()),
            returned_session_token.or(resolved.session_token.as_deref()),
        );
    }
    json!({"jsonrpc":"2.0","id":id,"result":{"content":[{"type":"text","text":serde_json::to_string(&frames).unwrap()}]}})
}

fn tool_error(id: Value, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"result":{"isError":true,"content":[{"type":"text","text":message}]}})
}
