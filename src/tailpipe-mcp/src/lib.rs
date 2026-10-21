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

/// The MCP server-level pitch, shown to the client once at `initialize` --
/// the same mechanism chat-mcp and decisions-mcp already carry as their own
/// `INSTRUCTIONS` consts. Unlike those two, tailpipe-mcp shipped with none at
/// all (not a deliberate omission the way ai-text-editor-mcp's own
/// env-var-gated default is; just never added).
const INSTRUCTIONS: &str = "A less/tail for agents: pipe a running command's stdout into a named \
stream and read/search/tail it back by id, from anywhere, without having been the process that \
started it. Piping in (tailpipe-client-rs ingest --stream NAME) is shell-only -- there is no tool \
for that half -- but every read is a typed call here: list_streams names what is active, read takes \
an inclusive id range, search runs exact or regex, wait blocks (bounded by timeout_seconds) until a \
line past since arrives, and save snapshots a stream to gzip without evicting it. A stream lives \
only in server memory and is evicted -- snapshotted first, nothing lost -- after 15 minutes with no \
read or write. Not for durable, cross-session message history (use the chat skill) or for editing a \
file in place (use ai-text-editor).";

/// Unix socket paths have a small platform-defined limit (`sun_path` is 104
/// bytes on macOS, 108 on Linux, the null terminator included) -- a real
/// bind/connect failure ("path must be shorter than SUN_LEN"), not a
/// hypothetical: measured live against a deeply nested $HOME (a disposable
/// per-run HOME under a benchmark harness) at 126 bytes, 18 over the Linux
/// limit (B5). Conservative bound, leaving room for the socket's own
/// filename under the computed root.
const SUN_PATH_SAFE_LIMIT: usize = 90;

fn fits(path: &Path) -> bool {
    !cfg!(unix) || path.to_string_lossy().len() <= SUN_PATH_SAFE_LIMIT
}

/// Distinguishes one preferred root from another in the shared `/tmp`
/// fallback namespace, the same job chat-client-rs's own `state_tag` does for
/// its control socket: a collision costs a wrong-server connection attempt
/// (refused by the protocol, not a security property), not silent data
/// sharing between unrelated homes.
fn root_tag(preferred: &Path) -> String {
    // FNV-1a, matching chat-client-rs's own short-path fallback tag.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in preferred.as_os_str().as_encoded_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    format!("{:08x}", (hash >> 32) as u32)
}

/// Where the endpoint goes when the preferred, $HOME-rooted path would not
/// fit a unix socket address. `/tmp` rather than `std::env::temp_dir()`,
/// because a long `$TMPDIR` is the same kind of condition that got us here;
/// keyed by the preferred path's own tag so two distinct homes still resolve
/// to two distinct fallback endpoints instead of colliding on one.
fn short_endpoint(preferred: &Path) -> PathBuf {
    PathBuf::from("/tmp")
        .join(format!("tsch-ai-skills-tailpipe-{}", root_tag(preferred)))
        .join("tailpipe.sock")
}

/// The pure decision, taking the `$TAILPIPE_HOME` candidate explicitly
/// rather than reading it from the environment -- lets a test compute the
/// exact same answer for a home it chose itself, without mutating this
/// process's own env (tests run as parallel threads in one process, where an
/// env write lands under a sibling; the same reason planning-server's own
/// `endpoint::resolve` takes its runtime dir as a parameter).
fn resolve_endpoint_from(home: &Path) -> PathBuf {
    let preferred = home.join("tailpipe.sock");
    if fits(&preferred) {
        preferred
    } else {
        short_endpoint(&preferred)
    }
}

/// `$TAILPIPE_ENDPOINT` if set (trusted as-is: an explicit override is the
/// caller's own business), else the same `$TAILPIPE_HOME`-rooted default
/// path convention tailpipe-server-rs's own --snapshot-dir default uses,
/// with `tailpipe.sock` as the endpoint file name -- falling back to a short,
/// tagged path under `/tmp` when that preferred path would not fit a unix
/// socket address (B5).
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
    resolve_endpoint_from(&home)
}

#[cfg(test)]
mod endpoint_tests {
    use super::*;

    #[test]
    fn a_short_home_keeps_the_preferred_path() {
        let home = Path::new("/home/alice/.config/tsch-ai-skills/tailpipe");
        let resolved = resolve_endpoint_from(home);
        assert_eq!(resolved, home.join("tailpipe.sock"));
    }

    #[test]
    fn a_long_home_falls_back_to_a_short_tagged_tmp_path() {
        // The exact path this bug was filed from: a disposable per-cell HOME
        // under the skill-adoption benchmark harness.
        let home = Path::new(
            "/home/tschallacka/.cache/ai-skills-benchmark/skill-adoption/cell.XXXXXXXXXX/home/.config/tsch-ai-skills/tailpipe",
        );
        assert!(
            !fits(&home.join("tailpipe.sock")),
            "fixture must actually exceed the safe limit, or this test proves nothing"
        );
        let resolved = resolve_endpoint_from(home);
        assert!(
            fits(&resolved),
            "fallback must itself always fit: {}",
            resolved.display()
        );
        assert!(resolved.starts_with("/tmp"));
        assert_eq!(resolved.file_name().unwrap(), "tailpipe.sock");
    }

    #[test]
    fn two_distinct_long_homes_fall_back_to_two_distinct_endpoints() {
        let a = Path::new("/home/alice/very/deeply/nested/workspace/root/for/testing/.config/tsch-ai-skills/tailpipe");
        let b = Path::new("/home/bob/very/deeply/nested/workspace/root/for/testing/.config/tsch-ai-skills/tailpipe");
        assert_ne!(resolve_endpoint_from(a), resolve_endpoint_from(b));
    }

    #[test]
    fn the_same_long_home_resolves_to_the_same_endpoint_every_time() {
        let home = Path::new(
            "/home/tschallacka/.cache/ai-skills-benchmark/skill-adoption/cell.XXXXXXXXXX/home/.config/tsch-ai-skills/tailpipe",
        );
        assert_eq!(resolve_endpoint_from(home), resolve_endpoint_from(home));
    }
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
            "serverInfo":{"name":"tailpipe","version":"0.1.0"},
            "instructions": INSTRUCTIONS}}),
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

/// B3: the schema used to declare every argument -- including this one's --
/// as `"type": "string"` unconditionally, so a caller honoring that schema
/// (Claude Code among them) sent a JSON string for from/to/since/
/// timeout_seconds, which `Value::as_u64` never matches even when the
/// string holds a parseable integer. Accepting a numeric string here too,
/// on top of fixing the schema's own types below, means a client that still
/// sends a string (an older schema snapshot, a stricter MCP client that
/// prefers over-wide inputs) keeps working rather than failing again the
/// same way.
fn u64_arg(arguments: &Value, key: &str) -> Option<u64> {
    match arguments.get(key) {
        Some(Value::Number(number)) => number.as_u64(),
        Some(Value::String(text)) => text.parse().ok(),
        _ => None,
    }
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

/// B3: the JSON schema type for an argument, not just its name and doc --
/// the earlier shape stamped every argument "string" regardless, so a
/// numeric one (from/to/since/timeout_seconds) advertised a type its own
/// handler could never satisfy from a schema-honoring caller.
#[derive(Clone, Copy)]
enum ArgType {
    String,
    Integer,
}

impl ArgType {
    fn as_json_type(self) -> &'static str {
        match self {
            ArgType::String => "string",
            ArgType::Integer => "integer",
        }
    }
}

type Arg = (&'static str, &'static str, ArgType);
type ToolDef = (
    &'static str,
    &'static str,
    &'static [Arg],
    &'static [&'static str],
);

fn tool_definitions() -> Vec<Value> {
    let defs: &[ToolDef] = &[
        ("list_streams", "List the server's currently active stream names.", &[], &[]),
        (
            "read",
            "Read a named stream's lines in an inclusive id range.",
            &[
                ("stream", "the stream name", ArgType::String),
                ("from", "the first id, inclusive", ArgType::Integer),
                ("to", "the last id, inclusive", ArgType::Integer),
            ],
            &["stream", "from", "to"],
        ),
        (
            "search",
            "Run an exact-text or regex search against a named stream's retained lines.",
            &[
                ("stream", "the stream name", ArgType::String),
                ("mode", "exact or regex", ArgType::String),
                ("query", "the text or pattern to search for", ArgType::String),
            ],
            &["stream", "mode", "query"],
        ),
        (
            "wait",
            "Block (bounded by timeout_seconds) until stream has a line past since, then return it.",
            &[
                ("stream", "the stream name", ArgType::String),
                ("since", "the cursor id to wait past (default 0)", ArgType::Integer),
                (
                    "timeout_seconds",
                    "how long to wait before giving up (default 30)",
                    ArgType::Integer,
                ),
            ],
            &["stream"],
        ),
        (
            "save",
            "Trigger a gzip snapshot of a named stream without evicting it.",
            &[("stream", "the stream name", ArgType::String)],
            &["stream"],
        ),
    ];
    defs.iter()
        .map(|(name, description, arguments, required)| {
            let mut properties = Map::new();
            for (key, doc, arg_type) in arguments.iter() {
                properties.insert(
                    (*key).to_string(),
                    json!({"type": arg_type.as_json_type(), "description": doc}),
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
    fn initialize_reports_a_non_empty_instructions_pitch() {
        let response = handle(json!({"jsonrpc":"2.0","id":1,"method":"initialize"}));
        assert_eq!(response["result"]["serverInfo"]["name"], "tailpipe");
        let instructions = response["result"]["instructions"]
            .as_str()
            .expect("instructions must be a string");
        assert!(!instructions.is_empty());
        assert!(instructions.contains("tailpipe-client-rs ingest"));
    }

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

    /// B3: read and wait's numeric arguments must advertise a type their
    /// own handler can actually be sent -- a schema-honoring caller sends
    /// what the schema says, and "string" here meant from/to/since/
    /// timeout_seconds could never parse via `Value::as_u64`.
    #[test]
    fn reads_and_waits_numeric_arguments_are_typed_integer_not_string() {
        let response = handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}));
        let tools = response["result"]["tools"].as_array().unwrap();
        let schema_type = |tool: &str, arg: &str| {
            tools
                .iter()
                .find(|t| t["name"] == tool)
                .and_then(|t| t["inputSchema"]["properties"][arg]["type"].as_str())
                .map(str::to_string)
        };
        assert_eq!(schema_type("read", "from"), Some("integer".into()));
        assert_eq!(schema_type("read", "to"), Some("integer".into()));
        assert_eq!(schema_type("read", "stream"), Some("string".into()));
        assert_eq!(schema_type("wait", "since"), Some("integer".into()));
        assert_eq!(
            schema_type("wait", "timeout_seconds"),
            Some("integer".into())
        );
        assert_eq!(schema_type("search", "mode"), Some("string".into()));
        assert_eq!(schema_type("search", "query"), Some("string".into()));
    }

    #[test]
    fn u64_arg_accepts_a_real_number_and_a_numeric_string_alike() {
        assert_eq!(u64_arg(&json!({"from": 3}), "from"), Some(3));
        assert_eq!(u64_arg(&json!({"from": "3"}), "from"), Some(3));
    }

    #[test]
    fn u64_arg_refuses_a_non_numeric_string_and_a_missing_key() {
        assert_eq!(u64_arg(&json!({"from": "not a number"}), "from"), None);
        assert_eq!(u64_arg(&json!({}), "from"), None);
    }
}
