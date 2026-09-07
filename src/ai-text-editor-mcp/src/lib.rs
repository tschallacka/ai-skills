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

/// Every argument the adapter consumes out of `arguments` itself: it is
/// stripped before the rest becomes the server payload, so a schema-following
/// client can only ever send it if every tool declares it —
/// `additionalProperties` is false, and an MCP harness that validates will
/// refuse the call outright rather than pass it through.
///
/// `call_tool` strips exactly this list and `routing()` declares exactly this
/// list. Binding the two to one const is the actual fix: B195 (empty
/// inputSchemas), B197 (dropped expected_revision), B211 (page's `historical`)
/// and B217 (`open`'s document_mode and normalize_nfc) were four instances of
/// one class — the request builder forwarded a key the advertised schema did
/// not offer. A fifth per-tool patch would not have stopped a sixth.
///
/// `file` is not here: the server routes on it, so it stays in the payload.
/// `expected_revision` is not here either — it is declared on the mutating
/// tools only, and `mutating_required` is what pins that.
pub const ADAPTER_ARGUMENTS: &[&str] = &[
    "endpoint",
    "agent",
    "session",
    "document_mode",
    "normalize_nfc",
    "idle_timeout_seconds",
    "acknowledge_create_parents",
    "takeover_stale_endpoint",
    "auth_token",
    "session_token",
];

/// The advertised schema for one `ADAPTER_ARGUMENTS` key. Exhaustive on
/// purpose: adding a key to the const without describing it here does not
/// compile away quietly, it panics the schema test.
fn adapter_argument(key: &str) -> Value {
    match key {
        "endpoint" => string(
            "Explicit editor endpoint (unix:/path or host:port); wins over discovery.",
        ),
        "agent" => string(
            "Agent identity used to reconnect to this agent's running workspace.",
        ),
        "session" => string("Session identity; same resolution as agent."),
        "document_mode" => string(
            "Document mode for a server this call starts: text_utf8 (default), raw_bytes, or hex_view (16-byte rows). It shapes only a newly started server — when a workspace already serves the file, the tab reports what it actually is.",
        ),
        "normalize_nfc" => boolean(
            "Normalize the document to Unicode NFC when this call starts the server. Shapes only a newly started server; `restore` answers not_normalized on a tab opened without it.",
        ),
        "idle_timeout_seconds" => int(
            "Idle seconds after which a server this call starts shuts itself down.",
        ),
        "acknowledge_create_parents" => boolean(
            "Confirms that a path whose parent directory does not exist is meant as typed. Without it such an open is refused with the missing directory named and nothing is created; with it the directory chain is created and the tab opens.",
        ),
        "takeover_stale_endpoint" => boolean(
            "Confirms that the process recorded as owning a stale endpoint has been verified gone, so a server this call starts may replace it. Without it such a start is refused with the recorded pid and generation named and nothing is replaced; this is capability 12's explicit takeover.",
        ),
        "auth_token" => string(
            "Shared secret required by a server reached over a loopback TCP endpoint.",
        ),
        "session_token" => string(
            "Server-issued tab token, supplied explicitly instead of the one discovery cached.",
        ),
        other => panic!("ADAPTER_ARGUMENTS lists {other} with no advertised schema"),
    }
}

/// The protocol method a tool name addresses. Only one differs: `resolve` is
/// the tool, `resolve_external` the method, and `call_tool` maps it the same
/// way — a second spelling of that mapping is how the schema and the router
/// would come apart.
fn server_method(tool: &str) -> &str {
    if tool == "resolve" {
        "resolve_external"
    } else {
        tool
    }
}

fn tool_definitions() -> Vec<Value> {
    let routing = || {
        let mut properties: ToolProperties = Vec::from([
            (
                "file",
                string(
                    "Path served by this request; routes to that file's own tab in the agent's workspace, opening it if the workspace does not have it yet.",
                ),
            ),
            // T96: declared on every tool, including the job verbs, because
            // the point is that an id is sufficient addressing for all of
            // them. Not an ADAPTER_ARGUMENTS entry: the server routes on it,
            // so it stays in the payload the way `file` does.
            (
                "tab_id",
                string(
                    "The tab_id a previous answer reported, and addressing enough on its own: with it, no file or endpoint is needed for any verb. Wins over file and tab_path, and is refused by name (tab_unknown) rather than falling back to some other tab if it names none.",
                ),
            ),
            // T99: on every tool, because the ladder applies to every
            // answer. Not an ADAPTER_ARGUMENTS entry: the server consumes it,
            // so it stays in the payload like `file` and `tab_id`.
            (
                "verbosity",
                int(
                    "How much of the answer to return: 0 the result and the tab that gave it and nothing else, 1 (default) adds what verification and the next step need - revision, dirty, the tab's mode, the resolved edit span, completeness, and a search's pager key and count, 2 adds navigation - cursors, byte windows, block paging, undo depths, 3 everything. A level outside 0-3 is refused by name rather than clamped. capabilities and resources ignore it, their payload being metadata by definition, and a refusal always carries its full code, message and recovery choices whatever the level.",
                ),
            ),
            // T97: the recovery for an agent that lost the id.
            (
                "tab_path",
                string(
                    "A filename, or a trailing run of path components, naming an open tab in this agent's workspace - the recovery when the tab_id is lost. Matched on component boundaries, not as a substring. Naming several tabs is refused with tab_ambiguous and the candidates with their tab_ids; naming none with tab_unmatched and the open tabs, so the next attempt is informed rather than another guess.",
                ),
            ),
        ]);
        properties.extend(
            ADAPTER_ARGUMENTS
                .iter()
                .map(|key| (*key, adapter_argument(key))),
        );
        properties
    };
    let mut tools: Vec<ToolSpec> = Vec::new();
    tools.push(("open", "Inspect the tab path, document mode, revision, size, and cursors, and get the revision a mutation must carry. Opens the file if the workspace does not have it yet, starting a server when none runs, and document_mode chooses the mode of the tab it opens whether or not a workspace is already running.", Vec::from([
        // B238: this description is the whole point of the fix. The mode used
        // to be a property of the SERVER, so the honest schema had to say it
        // "shapes only a newly started server" — and since B225 made every
        // verb autostart and reconnect, that made a raw or hex tab reachable
        // only on an agent's very first open.
        ("document_mode", string("Mode of the tab this call opens: text_utf8 (default), raw_bytes, or hex_view (16-byte rows). A property of the tab, not of the workspace, so it applies to a file added to an already-running workspace as much as to the first one. Reopening a tab that already exists under a DIFFERENT mode is refused with document_mode_conflict: a tab's mode is fixed for its lifetime because its buffer, index and coordinates all committed to one reading of the bytes - close it and open it again.")),
    ]), vec![]));
    tools.push(("capabilities", "Inspect the machine-readable protocol modes, coordinate rules, defaults, resource limits, and transports. Answers from the running server when one is reachable, from compiled-in defaults (marked source: client_default) otherwise.", Vec::new(), vec![]));
    tools.push(("resources", "Inspect available memory, server overhead, working-set recommendation, and large-file threshold.", Vec::new(), vec![]));
    tools.push((
        "history",
        "Inspect undo/redo depths and journal sequence without changing the tab.",
        Vec::new(),
        vec![],
    ));
    tools.push(("read", "Read the current document or a bounded byte/line range. Line ranges are inclusive on text tabs; byte ranges half-open on raw and hex tabs; offset/length is a byte window snapped to UTF-8 boundaries.", { let mut p: ToolProperties = Vec::new(); p.extend(Vec::from([
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
            let mut p: ToolProperties = Vec::new();
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
        "Replace a span with text or base64 bytes; preserve the revision guard. Address the span three ways: offset plus delete_len in bytes, range_start_line/range_end_line (inclusive 1-based whole lines, the last line's newline included, so replacing with no text deletes the lines outright), or range_start_byte/range_end_byte (half-open, exactly what a search hit reports as byte_start/byte_end, so a span across two hits is those two numbers copied across). Pass expected_text to have the server verify the bytes at the span before deleting them.",
        {
            let mut p: ToolProperties = Vec::new();
            p.extend(Vec::from([
                (
                    "offset",
                    int("Byte offset; omitted means the cursor position."),
                ),
                (
                    "cursor_id",
                    int("Numeric cursor to replace at when offset is omitted."),
                ),
                ("delete_len", int("Byte length to delete before inserting. Omit it when expected_text names the span: its own length is then the length, so there is no arithmetic to get wrong.")),
                (
                    "range_start_line",
                    int("Inclusive first line of a line-range replace (text tabs). Needs range_end_line, and may not be combined with offset, delete_len or cursor_id."),
                ),
                (
                    "range_end_line",
                    int("Inclusive last line of a line-range replace; its newline goes with it, so replacing with no text deletes the lines outright."),
                ),
                (
                    "range_start_byte",
                    int("Inclusive first byte of a byte-range replace — a search hit's byte_start. Needs range_end_byte."),
                ),
                (
                    "range_end_byte",
                    int("Exclusive last byte of a byte-range replace — a search hit's byte_end, so a span across two hits needs no arithmetic."),
                ),
                (
                    "expected_text",
                    string("The bytes the caller believes are at the span. Verified BEFORE anything is deleted and refused by name on mismatch, which the revision guard cannot do: a revision proves the document has not moved since you read it, not that your length still matches the text there — an edit of your own that changed that text's length leaves the revision perfectly current and the length wrong."),
                ),
                (
                    "expected_bytes_base64",
                    string("expected_text for a raw or hex tab, or for bytes that are not UTF-8. Pass one of the two, not both."),
                ),
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
    tools.push(("large_edit", "Stream an acknowledged job-owned rewrite of a large file and atomically replace it.", { let mut p: ToolProperties = Vec::new(); p.extend(Vec::from([
        ("job_id", int("Queued job this edit executes.")),
        ("resume_token", string("The job's resume token; required, and never disclosed to callers without it.")),
        ("acknowledge_large_edit", boolean("Must be true; confirms the streamed rewrite cost.")),
        ("offset", int("Byte offset of the rewrite.")),
        ("delete_len", int("Byte length replaced.")),
        ("text", string("Replacement text.")),
        ("bytes_base64", string("Replacement bytes, alternative to text.")),
        ("expected_revision", revision_guard()),
    ])); p }, mutating_required()));
    tools.push(("begin_transaction", "Begin an explicit undo transaction; subsequent ordinary edits are grouped until end_transaction.", { let mut p: ToolProperties = Vec::new(); p.extend(Vec::from([("expected_revision", revision_guard())])); p }, mutating_required()));
    tools.push((
        "end_transaction",
        "Close the explicit undo transaction and commit its grouped undo step.",
        {
            let mut p: ToolProperties = Vec::new();
            p.extend(Vec::from([("expected_revision", revision_guard())]));
            p
        },
        mutating_required(),
    ));
    tools.push(("restore", "Turn off lossless NFC presentation; refuses with not_normalized when the tab never normalized, and refuses when edits made restoration lossy.", { let mut p: ToolProperties = Vec::new(); p.extend(Vec::from([("expected_revision", revision_guard())])); p }, mutating_required()));
    tools.push((
        "undo",
        "Undo one server history transaction; the server returns the new revision.",
        {
            let mut p: ToolProperties = Vec::new();
            p.extend(Vec::from([("expected_revision", revision_guard())]));
            p
        },
        mutating_required(),
    ));
    tools.push((
        "redo",
        "Redo one server history transaction; the server returns the new revision.",
        {
            let mut p: ToolProperties = Vec::new();
            p.extend(Vec::from([("expected_revision", revision_guard())]));
            p
        },
        mutating_required(),
    ));
    tools.push((
        "save",
        "Atomically save the current working view after external-change resolution.",
        {
            let mut p: ToolProperties = Vec::new();
            p.extend(Vec::from([("expected_revision", revision_guard())]));
            p
        },
        mutating_required(),
    ));
    tools.push(("save_as", "Atomically create a new target file without changing the active tab; existing targets are refused.", { let mut p: ToolProperties = Vec::new(); p.extend(Vec::from([("target_path", string("New file to create; must not already exist."))])); p }, vec![]));
    tools.push((
        "close",
        "Close the tab; first obtain and then explicitly choose journal preservation or cleanup.",
        {
            let mut p: ToolProperties = Vec::new();
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
            let mut p: ToolProperties = Vec::new();
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
        // B237, found by binding this schema to the server's own key table:
        // `action` was advertised here as "build or inspect" and the handler
        // reads no such key — it always performs the complete scan and then
        // pages the blocks. A caller asking to inspect got a full rebuild and
        // no sign that its argument had been dropped.
        "Perform a complete scan of the lazy line/byte index at an explicit granularity, persist it, and page the resulting blocks.",
        {
            let mut p: ToolProperties = Vec::new();
            p.extend(Vec::from([
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
            let mut p: ToolProperties = Vec::new();
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
    tools.push(("page", "Page a previous search or index result set by its pager key. Pages are refetched after any write; a stale generation is refused by name, and `historical` reads the persisted result as it was at its source revision.", { let mut p: ToolProperties = Vec::new(); p.extend(Vec::from([
        ("pager_key", string("Pager key from a previous search or index response.")),
        ("offset", int("Zero-based match offset to resume from.")),
        ("limit", int("Matches to return.")),
        // B211: the transport already forwards this key and the server
        // honours it; omitting it from the published schema made the
        // documented stale-result escape undiscoverable to a
        // schema-following client.
        ("historical", boolean("Replay the persisted result set as it was recorded, accepting that it is stale, instead of refusing a post-edit page.")),
    ])); p }, vec![]));
    // B237, found by binding this schema to the server's own key table:
    // `pager_key` and `historical` were advertised here — "re-page instead of
    // rescanning", "replay the stored result set rather than rescanning" — and
    // the search handler reads neither. Both are `page`'s keys. A caller that
    // followed this schema to avoid a rescan got a full rescan and a brand-new
    // result set, with nothing saying its argument had been dropped. The
    // description now points at the verb that does page.
    tools.push(("search", "Run exactly one explicit search mode and receive an immutable result id and pager key. Neither offset nor pager_key is a search argument: a fresh search always rescans, and re-paging or replaying an existing result set is `page`, with that pager_key.", { let mut p: ToolProperties = Vec::new(); p.extend(Vec::from([
        ("mode", string("exact_text, exact_bytes, wildcard, shell_wildcard, path_wildcard, regex_rust, regex_pcre2, fuzzy_edit, fuzzy_subsequence, fuzzy_token, fuzzy_ngram, fuzzy_phonetic, fuzzy_soundex.")),
        ("query", string("Search query, interpreted by mode.")),
        ("query_base64", string("Base64 query, alternative to query.")),
        ("limit", int("Preview matches to return (default 4).")),
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
            let mut p: ToolProperties = Vec::new();
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
    tools.push(("job_poll", "Read the current state, progress, and result of a job. Requires the resume_token issued at start; the token is never disclosed to a caller without it.", Vec::new().into_iter().chain(Vec::from([("job_id", int("Job id.")), ("resume_token", string("Token from job_start; required."))])).collect(), vec!["job_id", "resume_token"]));
    tools.push((
        "job_progress",
        "Publish truthful progress for work the driving agent owns.",
        {
            let mut p: ToolProperties = Vec::new();
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
    tools.push(("job_complete", "Publish a terminal result for work the driving agent owns. The result must be a JSON array of result frames; a single object is accepted and wrapped, never dropped.", { let mut p: ToolProperties = Vec::new(); p.extend(Vec::from([
        ("job_id", int("Job id.")),
        ("resume_token", string("Token from job_start; required.")),
        ("result", json!({"description": "Array of result frames (a bare object is wrapped into one)."})),
    ])); p }, vec!["job_id", "resume_token"]));
    tools.push((
        "job_cancel",
        "Cancel a non-terminal job; cancellation wins races with completion.",
        Vec::new()
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
            let mut p: ToolProperties = Vec::new();
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
        Vec::new()
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
            // B237: the advertised payload keys are not a second list. They are
            // `ai_text_editor::verbs`, the table the server's door refuses
            // against, so a key one surface knows about and the other does not
            // cannot be written — an extra here panics as unadvertised, a
            // missing one panics as undescribed. `ADAPTER_ARGUMENTS` did this
            // for the arguments the adapter consumes (B217); this is the same
            // binding for the arguments the server consumes.
            let method = server_method(name);
            let described: std::collections::HashMap<&str, Value> = props.into_iter().collect();
            let mut properties = serde_json::Map::new();
            for (key, value) in routing() {
                properties.insert(key.to_string(), value);
            }
            let accepted = ai_text_editor::verbs::extra_payload_keys(method)
                .unwrap_or_else(|| panic!("{name} maps to {method}, which the server does not dispatch"));
            for key in accepted {
                let value = described.get(key).cloned().unwrap_or_else(|| {
                    panic!("{name} accepts the payload key {key} and describes no schema for it")
                });
                properties.insert((*key).to_string(), value);
            }
            // The revision guard rides the envelope, not the payload, so it is
            // the one described key with no row in the table.
            if let Some(value) = described.get("expected_revision") {
                properties.insert("expected_revision".into(), value.clone());
            }
            for key in described.keys() {
                assert!(
                    *key == "expected_revision" || accepted.contains(key),
                    "{name} advertises {key}, which {method} does not read - the server would refuse it as an unknown argument"
                );
            }
            json!({
                "name": name,
                "description": format!("{description} Either an endpoint, or a file plus an optional agent/session id to reconnect to that agent's already-running workspace, resolves the target. Naming a file opens that file's tab if the workspace does not have it yet, starting a server when none runs — on this tool as on open. The exception is the revision-guarded tools (insert, replace, large_edit, restore, undo, redo, save): on a file with no tab they are refused, because the revision they carry cannot have come from a tab that never existed. The server is authoritative and every mutation needs a current revision."),
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
    let method = server_method(name);
    let mut payload = arguments.as_object().cloned().unwrap_or_default();
    let file = payload
        .get("file")
        .and_then(Value::as_str)
        .map(PathBuf::from);
    let resolve_request = ResolveRequest {
        file: file.clone(),
        // Read, and left in the payload: the server routes on both.
        tab_id: payload
            .get("tab_id")
            .and_then(Value::as_str)
            .map(str::to_owned),
        tab_path: payload
            .get("tab_path")
            .and_then(Value::as_str)
            .map(str::to_owned),
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
        acknowledge_create_parents: payload
            .get("acknowledge_create_parents")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        takeover_stale_endpoint: payload
            .get("takeover_stale_endpoint")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        force_refresh: false,
    };
    let auth_token = payload
        .get("auth_token")
        .and_then(|value| value.as_str().map(str::to_owned));
    let session_token = payload
        .get("session_token")
        .and_then(|value| value.as_str().map(str::to_owned));
    // One sweep from the same const `tool_definitions` declares, so an
    // argument the adapter consumes and an argument a client is allowed to
    // send cannot come apart again (B217).
    let tab_mode = payload.get("document_mode").cloned();
    for key in ADAPTER_ARGUMENTS {
        payload.remove(*key);
    }
    // B238: `document_mode` is consumed twice on `open` and once everywhere
    // else. As an adapter argument it becomes the argv of a server this call
    // may start; as an `open` payload key it is the mode of the tab being
    // opened, which is the half that was missing — a second file added to a
    // running workspace inherited the server's startup mode and could never
    // be a raw or hex tab. Put back for `open` alone, so on any other verb the
    // server still refuses it as an argument that verb does not read.
    if method == "open" {
        if let Some(mode) = tab_mode {
            payload.insert("document_mode".into(), mode);
        }
    }
    // B175, made honest: accept both the envelope's wire name (`revision`)
    // and the documented `expected_revision`, and honour the schema the
    // adapter itself advertises — the guard is typed as a string, so "7"
    // must satisfy it exactly as 7 does. A value that is present but
    // unsatisfiable is refused by name here; dropping it silently surfaced
    // as the server's revision_required and told the caller nothing about
    // which argument failed.
    let revision = match payload
        .remove("expected_revision")
        .or_else(|| payload.remove("revision"))
    {
        None => None,
        Some(value) => match parse_revision_argument(&value) {
            Ok(revision) => Some(revision),
            Err(error) => return tool_error(id, &error),
        },
    };
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
        // T98, and the surface it matters most on: an MCP agent's context is
        // the forgetful one, so a successful call focusing the tab that served
        // it is what lets the next call name nothing at all.
        client::persist_focus(
            &resolve_request,
            &resolved.endpoint,
            auth_token.as_deref().or(resolved.auth_token.as_deref()),
            returned_session_token.or(resolved.session_token.as_deref()),
        );
    }
    // A refusal must look refused on the MCP wire too: the CLI exits
    // non-zero when an error frame is in the answer, and an MCP harness
    // reads that verdict from isError. Serving a refusal as an ordinary
    // success let a failed edit pass as an applied one.
    let mut result =
        json!({"content": [{"type": "text", "text": serde_json::to_string(&frames).unwrap()}]});
    if failed {
        result["isError"] = json!(true);
    }
    json!({"jsonrpc":"2.0","id":id,"result":result})
}

fn tool_error(id: Value, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"result":{"isError":true,"content":[{"type":"text","text":message}]}})
}

fn parse_revision_argument(value: &Value) -> Result<u64, String> {
    let digits = match value {
        Value::String(text) => text.trim().to_string(),
        Value::Number(number) => number.to_string(),
        other => {
            return Err(format!(
                "expected_revision {other} is not a revision number; read open or history first"
            ))
        }
    };
    digits.parse::<u64>().map_err(|_| {
        format!("expected_revision {value} is not a revision number; read open or history first")
    })
}

#[cfg(test)]
mod tests {
    use super::{handle, mutating_required, parse_revision_argument, ADAPTER_ARGUMENTS};
    use serde_json::{json, Value};

    fn tools() -> Vec<Value> {
        let answer = handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}));
        answer
            .pointer("/result/tools")
            .and_then(Value::as_array)
            .cloned()
            .expect("tools/list answers with an array")
    }

    fn properties(tool: &Value) -> &serde_json::Map<String, Value> {
        tool.pointer("/inputSchema/properties")
            .and_then(Value::as_object)
            .expect("every tool advertises an object of properties")
    }

    /// B217, and the whole class it belongs to (B195, B197, B211): a key the
    /// adapter consumes out of `arguments` but no tool declares cannot be
    /// sent at all — `additionalProperties` is false, so a validating client
    /// refuses the call before it is made, and a capability the surface has
    /// is unreachable from it.
    ///
    /// This walks `tools/list` against the adapter's own consumed-key list
    /// rather than checking one tool's schema, so the next argument added to
    /// `ADAPTER_ARGUMENTS` is covered without anyone remembering to add a
    /// case here.
    #[test]
    fn every_argument_the_adapter_consumes_is_declared_on_every_tool() {
        let tools = tools();
        assert!(
            tools.len() >= 28,
            "expected the full tool set, got {}",
            tools.len()
        );
        for tool in &tools {
            let name = tool.get("name").and_then(Value::as_str).unwrap_or("?");
            let properties = properties(tool);
            assert!(
                properties.contains_key("file"),
                "{name} does not declare `file`"
            );
            for key in ADAPTER_ARGUMENTS {
                assert!(
                    properties.contains_key(*key),
                    "{name} does not declare `{key}`, which call_tool strips from every request"
                );
            }
        }
    }

    /// The headline of B217 spelled out, so the entry's reproduction reads
    /// back from the test: `open` advertised exactly agent, endpoint, file
    /// and session, which put raw_bytes, hex_view and NFC normalization out
    /// of reach of every schema-following MCP client.
    #[test]
    fn open_declares_document_mode_and_normalize_nfc() {
        let tools = tools();
        let open = tools
            .iter()
            .find(|tool| tool.get("name").and_then(Value::as_str) == Some("open"))
            .expect("open is advertised");
        let properties = properties(open);
        assert!(properties.contains_key("document_mode"));
        assert!(properties.contains_key("normalize_nfc"));
        assert_eq!(
            properties["document_mode"]["type"], "string",
            "document_mode carries the mode name"
        );
        assert_eq!(
            properties["normalize_nfc"]["type"], "boolean",
            "normalize_nfc is a flag, not a string"
        );
    }

    /// A required key that is not in `properties` is required and unsendable
    /// at once — the shape B197 had.
    #[test]
    fn every_required_key_is_also_declared() {
        for tool in tools() {
            let name = tool.get("name").and_then(Value::as_str).unwrap_or("?");
            let required = tool
                .pointer("/inputSchema/required")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let properties = properties(&tool);
            for key in required {
                let key = key.as_str().unwrap_or_default();
                assert!(
                    properties.contains_key(key),
                    "{name} requires `{key}` and does not declare it"
                );
            }
        }
    }

    /// The mutating tools' revision guard is the one adapter-consumed
    /// argument that is deliberately not on every tool, so it is pinned
    /// where it does belong rather than left to the sweep above.
    #[test]
    fn the_revision_guard_is_declared_wherever_it_is_required() {
        let guard = mutating_required();
        assert_eq!(guard, vec!["expected_revision"]);
        let mut found = 0;
        for tool in tools() {
            let required = tool
                .pointer("/inputSchema/required")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            if required.iter().any(|key| key == "expected_revision") {
                let name = tool.get("name").and_then(Value::as_str).unwrap_or("?");
                assert!(
                    properties(&tool).contains_key("expected_revision"),
                    "{name} requires the revision guard and does not declare it"
                );
                found += 1;
            }
        }
        assert!(found >= 7, "expected the mutating tools, found {found}");
    }

    /// B251 and B252, the schema half. The server refusing these by name is
    /// pinned in cli_flow; this pins that the schema no longer OFFERS them,
    /// which is the half that matters to a schema-following client — it would
    /// otherwise be told to send an argument the server now rejects, and the
    /// fix would read as a regression.
    ///
    /// Not spelled out per tool by hand: `tool_definitions` builds every
    /// property list from `ai_text_editor::verbs`, and advertising a key that
    /// table does not carry panics at build time. So this asserts the
    /// consequence for the three keys the entries name, and the binding itself
    /// is what keeps the rest honest.
    #[test]
    fn no_tool_advertises_an_argument_its_verb_does_not_read() {
        let tools = tools();
        let named = |name: &str| {
            tools
                .iter()
                .find(|tool| tool.get("name").and_then(Value::as_str) == Some(name))
                .unwrap_or_else(|| panic!("{name} is advertised"))
        };
        // B251: both are `page` keys, and search reads neither. Advertising
        // them inverted their purpose — they exist to avoid a rescan.
        let search = properties(named("search"));
        for key in ["pager_key", "historical"] {
            assert!(
                !search.contains_key(key),
                "search must not advertise {key}, which only page reads"
            );
        }
        // They are real arguments, on the verb that does read them.
        let page = properties(named("page"));
        assert!(page.contains_key("pager_key") && page.contains_key("historical"));
        // B252: no verb reads this one at all, so it is deleted rather than
        // relocated.
        assert!(
            !properties(named("index")).contains_key("action"),
            "index must not advertise an action it never reads"
        );
        // The verbs that DO take an action still advertise it, so the deletion
        // was of a stale entry and not of the concept.
        for name in ["cursor", "resolve"] {
            assert!(
                properties(named(name)).contains_key("action"),
                "{name} reads action and must still advertise it"
            );
        }
    }

    #[test]
    fn a_string_revision_satisfies_the_guard() {
        // The advertised schema types expected_revision as a string; a
        // client that follows it must pass the server's guard exactly as a
        // numeric caller does. B175 was closed once without this pinned,
        // and every schema-following MCP client hit revision_required
        // again on the shipped build.
        assert_eq!(parse_revision_argument(&json!("0")), Ok(0));
        assert_eq!(parse_revision_argument(&json!(" 17 ")), Ok(17));
        assert_eq!(parse_revision_argument(&json!(17)), Ok(17));
    }

    #[test]
    fn an_unsatisfiable_revision_is_refused_by_name() {
        // Silence here was the defect: the argument vanished and the
        // server's revision_required then blamed the caller for omitting
        // the very value it had been given.
        for value in [
            json!(""),
            json!("seven"),
            json!(-1),
            json!(3.5),
            json!(null),
            json!(true),
        ] {
            let error = parse_revision_argument(&value).unwrap_err();
            assert!(
                error.starts_with("expected_revision"),
                "{value} refused with: {error}"
            );
        }
    }
}
