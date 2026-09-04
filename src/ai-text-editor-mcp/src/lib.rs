// MODE: DEV
// PACKAGE: PROD
//! Optional MCP adapter for the editor protocol.

use ai_text_editor::client::{self, ResolveRequest};
use ai_text_editor::protocol::Envelope;
use ai_text_editor::transport::request;
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

fn tool_definitions() -> Vec<Value> {
    [
        ("open", "Inspect the tab path, document mode, revision, size, and cursors."),
        ("capabilities", "Inspect the machine-readable protocol modes, coordinate rules, defaults, resource limits, and transports."),
        ("history", "Inspect undo/redo depths and journal sequence without changing the tab."),
        ("resources", "Inspect available memory, server overhead, working-set recommendation, and large-file threshold."),
        ("read", "Read the current document or a bounded byte/line range."),
        ("insert", "Insert text or base64 bytes at a byte offset; preserve the revision guard."),
        ("replace", "Replace a byte range with text or base64 bytes; preserve the revision guard."),
        ("large_edit", "Stream an acknowledged job-owned rewrite of a large file and atomically replace it."),
        ("begin_transaction", "Begin an explicit undo transaction; subsequent ordinary edits are grouped until end_transaction."),
        ("end_transaction", "Close the explicit undo transaction and commit its grouped undo step."),
        ("restore", "Turn off lossless NFC presentation; refuses when edits made restoration lossy."),
        ("undo", "Undo one server history transaction; the server returns the new revision."),
        ("redo", "Redo one server history transaction; the server returns the new revision."),
        ("save", "Atomically save the current working view after external-change resolution."),
        ("save_as", "Atomically create a new target file without changing the active tab; existing targets are refused."),
        ("close", "Close the tab; first obtain and then explicitly choose journal preservation or cleanup."),
        ("resolve", "Resolve an external change with reload, merge, keep, or acknowledged force_save."),
        ("index", "Build or inspect the lazy line/byte index, optionally with explicit granularity."),
        ("cursor", "Create, move, or inspect numeric cursors and navigation positions."),
        ("search", "Run exactly one explicit search mode and receive an immutable result id and pager key."),
        ("job_start", "Create a lifecycle record for agent-owned long work; this tool does not execute the work."),
        ("job_poll", "Read the current state, progress, result, and resume token status of a job."),
        ("job_progress", "Publish truthful progress for work the driving agent owns."),
        ("job_complete", "Publish a terminal result for work the driving agent owns."),
        ("job_cancel", "Cancel a non-terminal job; cancellation wins races with completion."),
        ("job_transfer", "Transfer ownership using the current resume token."),
        ("job_release", "Permanently release a job and invalidate its resume token."),
    ]
        .into_iter()
        .map(|(name, description)| json!({"name":name,"description":format!("{description} Either an endpoint, or a file (for open) plus an optional agent/session id to reconnect to that agent's already-running workspace, resolves the target; a new workspace-server is started automatically if `open` finds none. The server is authoritative and every mutation needs a current revision."),"inputSchema":{"type":"object","additionalProperties":true}}))
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
    };
    let resolved = match client::resolve(&resolve_request) {
        Ok(resolved) => resolved,
        Err(error) => return tool_error(id, &error),
    };
    payload.remove("endpoint");
    payload.remove("session");
    payload.remove("agent");
    payload.remove("document_mode");
    payload.remove("normalize_nfc");
    payload.remove("idle_timeout_seconds");
    let revision = payload.remove("revision").and_then(|value| value.as_u64());
    let auth_token = payload
        .remove("auth_token")
        .and_then(|value| value.as_str().map(str::to_owned))
        .or(resolved.auth_token.clone());
    let session_token = payload
        .remove("session_token")
        .and_then(|value| value.as_str().map(str::to_owned))
        .or(resolved.session_token.clone());
    let envelope = Envelope {
        version: ai_text_editor::PROTOCOL_VERSION,
        request_id: "mcp-1".into(),
        method: method.into(),
        revision,
        auth_token: auth_token.clone(),
        session_token,
        payload: Value::Object(payload),
    };
    match request(&resolved.endpoint, &envelope) {
        Ok(frames) => {
            let returned_session_token = frames
                .iter()
                .find(|frame| frame.get("type").and_then(Value::as_str) == Some("data"))
                .and_then(|frame| frame.pointer("/payload/session_token"))
                .and_then(Value::as_str);
            let _ = client::persist_cache(
                resolved.cache_path.as_deref(),
                &resolved.endpoint,
                auth_token.as_deref(),
                returned_session_token.or(resolved.session_token.as_deref()),
            );
            json!({"jsonrpc":"2.0","id":id,"result":{"content":[{"type":"text","text":serde_json::to_string(&frames).unwrap()}]}})
        }
        Err(error) => tool_error(id, &error),
    }
}

fn tool_error(id: Value, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"result":{"isError":true,"content":[{"type":"text","text":message}]}})
}
