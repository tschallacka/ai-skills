// MODE: DEV
// PACKAGE: PROD
//! MCP adapter for tailpipe: list_streams/read/search/wait/save as typed
//! tool calls, linking tailpipe-client-rs as a library (mirroring chat-mcp's
//! own shape). The endpoint is resolved by the adapter itself, not taken as
//! a tool argument -- no schema here carries a socket path, matching
//! chat-mcp's own philosophy that transport details are the adapter's
//! business, not a model's to choose.

use serde_json::{json, Map, Value};
use std::path::{Path, PathBuf};
use std::time::Duration;
use tailpipe_server_rs::protocol::{Request, Response, SearchMode};

/// `$TAILPIPE_ENDPOINT` if set, else the same `$TAILPIPE_HOME`-rooted
/// default path convention tailpipe-server-rs's own --snapshot-dir default
/// uses, with `tailpipe.sock` as the endpoint file name.
pub fn resolve_endpoint() -> PathBuf {
    if let Ok(endpoint) = std::env::var("TAILPIPE_ENDPOINT") {
        return PathBuf::from(endpoint);
    }
    let home = std::env::var("TAILPIPE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let config_home = std::env::var("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".to_string()))
                        .join(".config")
                });
            config_home.join("tsch-ai-skills").join("tailpipe")
        });
    home.join("tailpipe.sock")
}

fn request(endpoint: &Path, req: &Request) -> Result<Response, String> {
    tailpipe_client_rs::client::connect_and_request(endpoint, req)
        .map_err(|error| error.to_string())
}

// ─────────────────────────────────────────────────────────────────────────────
// list_streams
// ─────────────────────────────────────────────────────────────────────────────

fn list_streams_result(response: Response) -> Result<Value, String> {
    match response {
        Response::Streams { names } => Ok(json!({ "streams": names })),
        Response::Error { code, message } => Err(format!("{code}: {message}")),
        other => Err(format!("unexpected response: {other:?}")),
    }
}

pub fn list_streams(endpoint: &Path) -> Result<Value, String> {
    list_streams_result(request(endpoint, &Request::List)?)
}

// ─────────────────────────────────────────────────────────────────────────────
// read
// ─────────────────────────────────────────────────────────────────────────────

fn lines_result(response: Response) -> Result<Value, String> {
    match response {
        Response::Lines { lines } => Ok(json!({
            "lines": lines.into_iter().map(|line| json!({"id": line.id, "text": line.text})).collect::<Vec<_>>()
        })),
        Response::Error { code, message } => Err(format!("{code}: {message}")),
        other => Err(format!("unexpected response: {other:?}")),
    }
}

pub fn read(endpoint: &Path, stream: &str, from: u64, to: u64) -> Result<Value, String> {
    lines_result(request(
        endpoint,
        &Request::Read {
            stream: stream.to_string(),
            from,
            to,
        },
    )?)
}

// ─────────────────────────────────────────────────────────────────────────────
// search
// ─────────────────────────────────────────────────────────────────────────────

pub fn search(
    endpoint: &Path,
    stream: &str,
    mode: SearchMode,
    query: &str,
) -> Result<Value, String> {
    lines_result(request(
        endpoint,
        &Request::Search {
            stream: stream.to_string(),
            mode,
            query: query.to_string(),
        },
    )?)
}

// ─────────────────────────────────────────────────────────────────────────────
// wait
// ─────────────────────────────────────────────────────────────────────────────

fn wait_result(received: Option<Response>) -> Result<Value, String> {
    match received {
        None => Ok(json!({ "timed_out": true })),
        Some(Response::Lines { lines }) => {
            let line = lines.into_iter().next();
            match line {
                Some(line) => Ok(json!({"line": {"id": line.id, "text": line.text}})),
                None => Ok(json!({ "timed_out": true })),
            }
        }
        Some(other) => Err(format!("unexpected response: {other:?}")),
    }
}

/// Blocks (bounded by `timeout_seconds`) for a line past `since` on `stream`.
/// Spawns tailpipe_client_rs::client::tail on its own thread and races it
/// against the timeout with a channel, since tail's own long-poll loop has
/// no stop signal of its own -- the thread outlives a timed-out wait, which
/// is an accepted tradeoff for a tool that is called occasionally rather
/// than in a tight loop.
pub fn wait(
    endpoint: &Path,
    stream: &str,
    since: u64,
    timeout_seconds: u64,
) -> Result<Value, String> {
    let (sender, receiver) = std::sync::mpsc::channel();
    let endpoint = endpoint.to_path_buf();
    let stream = stream.to_string();
    std::thread::spawn(move || {
        let _ = tailpipe_client_rs::client::tail(&endpoint, &stream, since, move |response| {
            let _ = sender.send(response);
        });
    });
    let received = receiver
        .recv_timeout(Duration::from_secs(timeout_seconds))
        .ok();
    wait_result(received)
}

// ─────────────────────────────────────────────────────────────────────────────
// save
// ─────────────────────────────────────────────────────────────────────────────

fn saved_result(response: Response) -> Result<Value, String> {
    match response {
        Response::Saved { path } => Ok(json!({ "path": path })),
        Response::Error { code, message } => Err(format!("{code}: {message}")),
        other => Err(format!("unexpected response: {other:?}")),
    }
}

pub fn save(endpoint: &Path, stream: &str) -> Result<Value, String> {
    saved_result(request(
        endpoint,
        &Request::Save {
            stream: stream.to_string(),
        },
    )?)
}

// ─────────────────────────────────────────────────────────────────────────────
// MCP transport: JSON-RPC methods and tool dispatch, mirroring chat-mcp's
// own handle/tool_definitions/call_tool shape.
// ─────────────────────────────────────────────────────────────────────────────

pub fn handle(message: Value) -> Value {
    let id = message.get("id").cloned().unwrap_or(Value::Null);
    let method = message.get("method").and_then(Value::as_str).unwrap_or("");
    match method {
        "initialize" => json!({"jsonrpc":"2.0","id":id,"result":{
            "protocolVersion":"2025-06-18",
            "capabilities":{"tools":{}},
            "serverInfo":{"name":"tailpipe","version":"0.1.0"}}}),
        "notifications/initialized" => Value::Null,
        "tools/list" => json!({"jsonrpc":"2.0","id":id,"result":{"tools": tool_definitions()}}),
        "tools/call" => call_tool(id, message.get("params").cloned().unwrap_or_default()),
        _ => json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"method not found"}}),
    }
}

fn string_arg(arguments: &Value, key: &str) -> Option<String> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn u64_arg(arguments: &Value, key: &str) -> Option<u64> {
    arguments.get(key).and_then(Value::as_u64)
}

fn call_tool(id: Value, params: Value) -> Value {
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let endpoint = resolve_endpoint();

    let result = match name {
        "list_streams" => list_streams(&endpoint),
        "read" => {
            let stream = string_arg(&arguments, "stream").ok_or("read needs stream".to_string());
            let from = u64_arg(&arguments, "from").ok_or("read needs from".to_string());
            let to = u64_arg(&arguments, "to").ok_or("read needs to".to_string());
            match (stream, from, to) {
                (Ok(stream), Ok(from), Ok(to)) => read(&endpoint, &stream, from, to),
                (Err(message), _, _) | (_, Err(message), _) | (_, _, Err(message)) => Err(message),
            }
        }
        "search" => {
            let stream = string_arg(&arguments, "stream").ok_or("search needs stream".to_string());
            let mode = match string_arg(&arguments, "mode").as_deref() {
                Some("exact") => Ok(SearchMode::Exact),
                Some("regex") => Ok(SearchMode::Regex),
                _ => Err("search needs mode: exact or regex".to_string()),
            };
            let query = string_arg(&arguments, "query").ok_or("search needs query".to_string());
            match (stream, mode, query) {
                (Ok(stream), Ok(mode), Ok(query)) => search(&endpoint, &stream, mode, &query),
                (Err(message), _, _) | (_, Err(message), _) | (_, _, Err(message)) => Err(message),
            }
        }
        "wait" => {
            let stream = string_arg(&arguments, "stream").ok_or("wait needs stream".to_string());
            let since = u64_arg(&arguments, "since").unwrap_or(0);
            let timeout_seconds = u64_arg(&arguments, "timeout_seconds").unwrap_or(30);
            match stream {
                Ok(stream) => wait(&endpoint, &stream, since, timeout_seconds),
                Err(message) => Err(message),
            }
        }
        "save" => {
            let stream = string_arg(&arguments, "stream").ok_or("save needs stream".to_string());
            match stream {
                Ok(stream) => save(&endpoint, &stream),
                Err(message) => Err(message),
            }
        }
        other => Err(format!(
            "unknown tool: {other}. The tools are list_streams, read, search, wait and save."
        )),
    };

    match result {
        Ok(value) => json!({"jsonrpc":"2.0","id":id,"result":{"content":[
            {"type":"text","text": serde_json::to_string_pretty(&value).unwrap_or_default()}]}}),
        Err(message) => {
            json!({"jsonrpc":"2.0","id":id,"result":{"isError":true,"content":[{"type":"text","text":message}]}})
        }
    }
}

type ToolDef = (
    &'static str,
    &'static str,
    &'static [(&'static str, &'static str)],
    &'static [&'static str],
);

fn tool_definitions() -> Vec<Value> {
    let defs: &[ToolDef] = &[
        ("list_streams", "List the server's currently active stream names.", &[], &[]),
        (
            "read",
            "Read a named stream's lines in an inclusive id range.",
            &[
                ("stream", "the stream name"),
                ("from", "the first id, inclusive"),
                ("to", "the last id, inclusive"),
            ],
            &["stream", "from", "to"],
        ),
        (
            "search",
            "Run an exact-text or regex search against a named stream's retained lines.",
            &[
                ("stream", "the stream name"),
                ("mode", "exact or regex"),
                ("query", "the text or pattern to search for"),
            ],
            &["stream", "mode", "query"],
        ),
        (
            "wait",
            "Block (bounded by timeout_seconds) until stream has a line past since, then return it.",
            &[
                ("stream", "the stream name"),
                ("since", "the cursor id to wait past (default 0)"),
                ("timeout_seconds", "how long to wait before giving up (default 30)"),
            ],
            &["stream"],
        ),
        (
            "save",
            "Trigger a gzip snapshot of a named stream without evicting it.",
            &[("stream", "the stream name")],
            &["stream"],
        ),
    ];
    defs.iter()
        .map(|(name, description, arguments, required)| {
            let mut properties = Map::new();
            for (key, doc) in arguments.iter() {
                properties.insert(
                    (*key).to_string(),
                    json!({"type": "string", "description": doc}),
                );
            }
            json!({
                "name": name,
                "description": description,
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

#[cfg(test)]
mod tests {
    use super::*;
    use tailpipe_server_rs::protocol::Line;

    #[test]
    fn list_streams_result_shapes_a_stub_response() {
        let response = Response::Streams {
            names: vec!["a".into(), "b".into()],
        };
        let value = list_streams_result(response).unwrap();
        assert_eq!(value, json!({"streams": ["a", "b"]}));
    }

    #[test]
    fn lines_result_shapes_a_stub_response() {
        let response = Response::Lines {
            lines: vec![Line {
                id: 1,
                text: "hello".into(),
            }],
        };
        let value = lines_result(response).unwrap();
        assert_eq!(value, json!({"lines": [{"id": 1, "text": "hello"}]}));
    }

    #[test]
    fn saved_result_shapes_a_stub_response() {
        let response = Response::Saved {
            path: "/tmp/s-1.gz".into(),
        };
        let value = saved_result(response).unwrap();
        assert_eq!(value, json!({"path": "/tmp/s-1.gz"}));
    }

    #[test]
    fn an_error_response_is_reported_as_an_error() {
        let response = Response::Error {
            code: "not_found".into(),
            message: "no such stream".into(),
        };
        assert_eq!(
            list_streams_result(response).unwrap_err(),
            "not_found: no such stream"
        );
    }

    #[test]
    fn wait_result_reports_a_delivered_line() {
        let received = Some(Response::Lines {
            lines: vec![Line {
                id: 2,
                text: "pushed".into(),
            }],
        });
        assert_eq!(
            wait_result(received).unwrap(),
            json!({"line": {"id": 2, "text": "pushed"}})
        );
    }

    #[test]
    fn wait_result_reports_a_timeout_as_timed_out_not_an_error() {
        assert_eq!(wait_result(None).unwrap(), json!({"timed_out": true}));
    }

    #[test]
    fn tools_list_names_all_five_tools() {
        let response = handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}));
        let names: Vec<&str> = response["result"]["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tool| tool["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            vec!["list_streams", "read", "search", "wait", "save"]
        );
    }
}
