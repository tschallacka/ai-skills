// MODE: DEV
// PACKAGE: PROD
//! MCP adapter for `interactive-shell`: PTY sessions as typed tool calls,
//! for a caller that speaks MCP instead of shelling out to
//! `interactive-shell`/`interactive-shell-input` directly. Every tool but
//! `start` is a thin wrapper around the exact same JSONL wire protocol those
//! two binaries already speak -- resolve a socket, `connect_in_directory`
//! (which itself picks Unix-domain or TCP transport, whichever a session
//! actually is), send one `{"v":1,"op":...}` request, collect every JSONL
//! reply line. `start` spawns the real `interactive-shell` binary (looked up
//! as a sibling of this adapter's own executable, exactly like
//! ai-text-editor-mcp's own server autostart) rather than reimplementing the
//! PTY/transport layer in-process: `run()`'s interrupt handler is
//! process-wide state (`INTERRUPTED` is one shared static), so two sessions
//! sharing this adapter's own process via threads would each think the
//! other's SIGINT/SIGTERM was its own -- a real child process is the only
//! isolation that avoids that.

use interactive_shell_core::{connect_in_directory, load_session, session_identity};
use serde_json::{json, Map, Value};
use std::io::{Read, Write};
use std::net::Shutdown;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub fn handle(message: Value) -> Value {
    let id = message.get("id").cloned().unwrap_or(Value::Null);
    let method = message.get("method").and_then(Value::as_str).unwrap_or("");
    match method {
        "initialize" => json!({"jsonrpc":"2.0","id":id,"result":{
            "protocolVersion":"2025-06-18",
            "capabilities":{"tools":{}},
            "serverInfo":{"name":"interactive-shell","version":"0.1.0"}}}),
        "notifications/initialized" => Value::Null,
        "tools/list" => json!({"jsonrpc":"2.0","id":id,"result":{"tools": tool_definitions()}}),
        "tools/call" => call_tool(id, message.get("params").cloned().unwrap_or_default()),
        _ => json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"method not found"}}),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// The advertised schema. One const declares every argument a tool may carry,
// one exhaustive match describes each, and `routing` names which tool takes
// which -- the same shape chat-mcp's own adapter uses, so an argument a tool
// does not accept is refused the same way there: at the CALLING binary's own
// schema, never silently passed through to something that ignores it.
// ─────────────────────────────────────────────────────────────────────────────

pub const TOOL_ARGUMENTS: &[&str] = &[
    "command",
    "session",
    "agent",
    "socket",
    "cols",
    "rows",
    "idle_timeout_seconds",
    "tcp",
    "text",
    "key",
    "ctrl",
    "alt",
    "shift",
    "hex",
    "query",
    "mode",
    "row_ranges",
    "contains",
    "timeout_ms",
    "x",
    "y",
    "button",
    "action",
    "target",
    "id",
    "label",
];

fn tool_argument(key: &str) -> Value {
    match key {
        "command" => {
            json!({"type":"array","items":{"type":"string"},"description":"The program and its arguments to run in the new PTY session, e.g. [\"bash\",\"--noprofile\",\"--norc\"]. Only start reads this."})
        }
        "session" => {
            json!({"type":"string","description":"A session id: start saves the socket/command/dimensions under it, and every other tool resolves the socket from it instead of a bare --socket path. Omit to fall back to this process's own INTERACTIVE_SHELL_AGENT/CODEX_AGENT_ID/AGENT_ID, if any."})
        }
        "agent" => {
            json!({"type":"string","description":"Same as session; either works, session wins if both are given."})
        }
        "socket" => {
            json!({"type":"string","description":"An explicit socket/discovery-file path, when you are not using session/agent. On start this is where the new session is bound; on every other tool it is where an existing one is reached."})
        }
        "cols" => {
            json!({"type":"integer","description":"Terminal width in columns. On start: the new PTY's width (default 80, 1..240). On resize: the new width to report to the running program."})
        }
        "rows" => {
            json!({"type":"integer","description":"Terminal height in rows. On start: the new PTY's height (default 24, 1..100). On resize: the new height to report to the running program."})
        }
        "idle_timeout_seconds" => {
            json!({"type":"integer","description":"start only: stop the session after this many seconds with no PTY output (default 300)."})
        }
        "tcp" => {
            json!({"type":"boolean","description":"start only: bind loopback TCP instead of a Unix domain socket, for a sandbox that runs the command but blocks AF_UNIX socket creation for it. A later restart of the same session/agent id remembers this with no need to repeat it."})
        }
        "text" => {
            json!({"type":"string","description":"Literal text to type (text tool) or bracketed-paste (paste tool)."})
        }
        "key" => {
            json!({"type":"string","description":"A named key (ENTER, CTRL-A, UP, F5, ...) or a single printable character."})
        }
        "ctrl" => json!({"type":"boolean","description":"combo only: hold Ctrl."}),
        "alt" => json!({"type":"boolean","description":"combo only: hold Alt."}),
        "shift" => json!({"type":"boolean","description":"combo only: hold Shift."}),
        "hex" => {
            json!({"type":"string","description":"raw only: an explicit byte sequence, non-empty even-length hexadecimal."})
        }
        "query" => {
            json!({"type":"string","description":"locate only: visible text to find; returns 1-based row/column matches without typing anything."})
        }
        "mode" => {
            json!({"type":"string","enum":["view","view-delta","rgbview","rgbview-delta"],"description":"view only: view (compact, numbered), view-delta (only rows changed since the previous view), rgbview (ANSI colors preserved), rgbview-delta (colored changed rows only). Default view."})
        }
        "row_ranges" => {
            json!({"type":"array","items":{"type":"string"},"description":"view/elements/markup only: 1-based row numbers or inclusive ranges, e.g. [\"3\",\"10-15\"]. Omit for every row."})
        }
        "contains" => {
            json!({"type":"string","description":"wait only: block until this text is visible on screen, then return a full snapshot."})
        }
        "timeout_ms" => {
            json!({"type":"integer","description":"wait only: how long to wait before answering that it never appeared. Default 30000."})
        }
        "x" => {
            json!({"type":"integer","description":"1-based column. mouse: required. click: required when target is at."})
        }
        "y" => {
            json!({"type":"integer","description":"1-based row. mouse: required. click: required when target is at."})
        }
        "button" => {
            json!({"type":"integer","description":"mouse/click only: 0 is the primary button, 1-7 are the remaining protocol button codes."})
        }
        "action" => {
            json!({"type":"string","enum":["down","up","move"],"description":"mouse only."})
        }
        "target" => {
            json!({"type":"string","enum":["id","label","at"],"description":"click only: which of id/label/x+y identifies the element to click."})
        }
        "id" => {
            json!({"type":"string","description":"click only, when target is id: an element id from elements/observe/markup."})
        }
        "label" => {
            json!({"type":"string","description":"click only, when target is label: an element's visible label from elements/observe/markup."})
        }
        other => unreachable!("TOOL_ARGUMENTS declares {other} with no schema"),
    }
}

type ToolSpec = (
    &'static str,
    &'static str,
    &'static [&'static str],
    &'static [&'static str],
);

fn routing() -> &'static [ToolSpec] {
    &[
        (
            "start",
            "Start a new interactive-shell session: allocates a real PTY and runs command in it. Use session or agent (or --socket) so every other tool can reach it without repeating configuration.",
            &["command", "session", "agent", "socket", "cols", "rows", "idle_timeout_seconds", "tcp"],
            &["command"],
        ),
        (
            "text",
            "Send literal text, as if typed.",
            &["text", "session", "agent", "socket"],
            &["text"],
        ),
        (
            "paste",
            "Send text as a bracketed paste, distinct from typed text for programs that treat the two differently.",
            &["text", "session", "agent", "socket"],
            &["text"],
        ),
        (
            "key",
            "Send one named key (ENTER, CTRL-A, UP, F5, PAGEDOWN, ...) or one printable character.",
            &["key", "session", "agent", "socket"],
            &["key"],
        ),
        (
            "combo",
            "Send a Ctrl/Alt/Shift combination not covered by a named key.",
            &["key", "ctrl", "alt", "shift", "session", "agent", "socket"],
            &["key"],
        ),
        (
            "raw",
            "Send an explicit byte sequence as hexadecimal, for a control sequence no named key covers.",
            &["hex", "session", "agent", "socket"],
            &["hex"],
        ),
        (
            "locate",
            "Find visible text and return its 1-based row/column matches, without sending any input.",
            &["query", "session", "agent", "socket"],
            &["query"],
        ),
        (
            "view",
            "A plain-text or ANSI-colored view of the current screen, optionally only certain rows or only rows changed since the last view.",
            &["mode", "row_ranges", "session", "agent", "socket"],
            &[],
        ),
        (
            "elements",
            "Only verified actionable elements (OSC 8 hyperlinks) and coordinate hints for visible text runs, with their ids/labels/coordinates.",
            &["row_ranges", "session", "agent", "socket"],
            &[],
        ),
        (
            "markup",
            "The screen re-encoded as lightweight HTML-like text: verified links as <a href>, highlighted runs as <span class=\"selected ...\">, aligned rows as <table>. Useful for a dense or unfamiliar screen where view/elements would need a lot of back-and-forth.",
            &["row_ranges", "session", "agent", "socket"],
            &[],
        ),
        (
            "observe",
            "The full structured screen snapshot: every row, cursor, styles, scrollback, and elements.",
            &["session", "agent", "socket"],
            &[],
        ),
        (
            "wait",
            "Block until visible text appears on screen (publishing intervening screen updates), then return a full snapshot with matched true or false.",
            &["contains", "timeout_ms", "session", "agent", "socket"],
            &["contains"],
        ),
        (
            "mouse",
            "Send an xterm mouse event at a 1-based row/column.",
            &["x", "y", "button", "action", "session", "agent", "socket"],
            &["x", "y", "button", "action"],
        ),
        (
            "click",
            "Click a verified element by id or label, or an arbitrary 1-based coordinate. Use only for an actionable element from elements/observe/markup, or when the TUI exposes only coordinate-based actions.",
            &["target", "id", "label", "x", "y", "button", "session", "agent", "socket"],
            &["target", "button"],
        ),
        (
            "resize",
            "Change both the PTY and the screen model's dimensions.",
            &["cols", "rows", "session", "agent", "socket"],
            &["cols", "rows"],
        ),
        (
            "shutdown",
            "End the session: stops the child and removes its socket/discovery file.",
            &["session", "agent", "socket"],
            &[],
        ),
    ]
}

fn tool_definitions() -> Vec<Value> {
    routing()
        .iter()
        .map(|(name, description, arguments, required)| {
            let mut properties = Map::new();
            for key in arguments.iter() {
                properties.insert((*key).to_string(), tool_argument(key));
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

// ─────────────────────────────────────────────────────────────────────────────
// Dispatch.
// ─────────────────────────────────────────────────────────────────────────────

fn call_tool(id: Value, params: Value) -> Value {
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));
    match dispatch(name, &arguments) {
        Ok(value) => json!({"jsonrpc":"2.0","id":id,"result":{"content":[
            {"type":"text","text": serde_json::to_string_pretty(&value).unwrap_or_default()}]}}),
        Err(message) => tool_error(id, &message),
    }
}

fn tool_error(id: Value, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"result":{"isError":true,"content":[{"type":"text","text":message}]}})
}

fn dispatch(name: &str, arguments: &Value) -> Result<Value, String> {
    if name == "start" {
        return start_session(arguments);
    }
    if !routing().iter().any(|(tool, ..)| *tool == name) {
        return Err(format!(
            "unknown tool: {name}. The tools are start, text, paste, key, combo, raw, locate, \
             view, elements, markup, observe, wait, mouse, click, resize and shutdown."
        ));
    }
    let socket = resolve_socket(arguments)?;
    let request = build_request(name, arguments)?;
    let responses = exchange(&socket, &request)?;
    Ok(json!({"responses": responses}))
}

// ─────────────────────────────────────────────────────────────────────────────
// Argument extraction.
// ─────────────────────────────────────────────────────────────────────────────

fn string_argument(arguments: &Value, key: &str) -> Option<String> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
}

fn u64_argument(arguments: &Value, key: &str) -> Option<u64> {
    arguments.get(key).and_then(Value::as_u64)
}

fn bool_argument(arguments: &Value, key: &str) -> bool {
    arguments.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn row_ranges_argument(arguments: &Value) -> Result<Vec<usize>, String> {
    let specs = arguments
        .get("row_ranges")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut rows = Vec::new();
    for spec in specs {
        let spec = spec
            .as_str()
            .ok_or_else(|| "row_ranges must be an array of strings".to_string())?;
        let (first, last) = spec.split_once('-').unwrap_or((spec, spec));
        let first: usize = first
            .parse()
            .map_err(|_| format!("invalid row_ranges entry: {spec}"))?;
        let last: usize = last
            .parse()
            .map_err(|_| format!("invalid row_ranges entry: {spec}"))?;
        if first == 0 || last == 0 || first > last {
            return Err(format!("invalid row_ranges range: {spec}"));
        }
        rows.extend(first..=last);
    }
    Ok(rows)
}

/// The same resolution `interactive-shell-input` does: an explicit socket
/// wins, otherwise session/agent (or this process's own
/// INTERACTIVE_SHELL_AGENT/CODEX_AGENT_ID/AGENT_ID) names a saved session.
fn resolve_socket(arguments: &Value) -> Result<PathBuf, String> {
    if let Some(socket) = string_argument(arguments, "socket") {
        return Ok(PathBuf::from(socket));
    }
    let session = string_argument(arguments, "session");
    let agent = string_argument(arguments, "agent");
    let id = session_identity(session.as_deref(), agent.as_deref()).ok_or_else(|| {
        "needs socket, session, or agent (or one of INTERACTIVE_SHELL_AGENT/CODEX_AGENT_ID/AGENT_ID set for this process)".to_string()
    })?;
    let saved = load_session(&id)?;
    let socket = saved
        .map(|session| session.socket)
        .ok_or_else(|| format!("no active session {id}"))?;
    if !socket.exists() {
        return Err(format!("session {id} is not active; start it first"));
    }
    Ok(socket)
}

fn build_request(name: &str, arguments: &Value) -> Result<Value, String> {
    let mut req = json!({"v":1,"op":name});
    match name {
        "text" | "paste" => {
            req["text"] = string_argument(arguments, "text")
                .ok_or_else(|| format!("{name} needs text"))?
                .into();
        }
        "locate" => {
            req["query"] = string_argument(arguments, "query")
                .ok_or_else(|| "locate needs query".to_string())?
                .into();
        }
        "key" => {
            req["key"] = string_argument(arguments, "key")
                .ok_or_else(|| "key needs key".to_string())?
                .into();
        }
        "raw" => {
            req["hex"] = string_argument(arguments, "hex")
                .ok_or_else(|| "raw needs hex".to_string())?
                .into();
        }
        "combo" => {
            req["key"] = string_argument(arguments, "key")
                .ok_or_else(|| "combo needs key".to_string())?
                .into();
            if bool_argument(arguments, "ctrl") {
                req["ctrl"] = true.into();
            }
            if bool_argument(arguments, "alt") {
                req["alt"] = true.into();
            }
            if bool_argument(arguments, "shift") {
                req["shift"] = true.into();
            }
        }
        "view" => {
            let mode = string_argument(arguments, "mode").unwrap_or_else(|| "view".to_string());
            if !matches!(
                mode.as_str(),
                "view" | "view-delta" | "rgbview" | "rgbview-delta"
            ) {
                return Err(format!("unknown view mode: {mode}"));
            }
            req["op"] = mode.into();
            req["rows"] = row_ranges_argument(arguments)?.into();
        }
        "elements" | "markup" => {
            req["rows"] = row_ranges_argument(arguments)?.into();
        }
        "observe" | "shutdown" => {}
        "wait" => {
            req["contains"] = string_argument(arguments, "contains")
                .ok_or_else(|| "wait needs contains".to_string())?
                .into();
            if let Some(timeout) = u64_argument(arguments, "timeout_ms") {
                req["timeout_ms"] = timeout.into();
            }
        }
        "mouse" => {
            req["x"] = u64_argument(arguments, "x")
                .ok_or_else(|| "mouse needs x".to_string())?
                .into();
            req["y"] = u64_argument(arguments, "y")
                .ok_or_else(|| "mouse needs y".to_string())?
                .into();
            req["button"] = u64_argument(arguments, "button")
                .ok_or_else(|| "mouse needs button".to_string())?
                .into();
            req["action"] = string_argument(arguments, "action")
                .ok_or_else(|| "mouse needs action".to_string())?
                .into();
        }
        "click" => {
            req["op"] = "click".into();
            match string_argument(arguments, "target")
                .ok_or_else(|| "click needs target: id, label, or at".to_string())?
                .as_str()
            {
                "id" => {
                    req["id"] = string_argument(arguments, "id")
                        .ok_or_else(|| "click target=id needs id".to_string())?
                        .into();
                }
                "label" => {
                    req["label"] = string_argument(arguments, "label")
                        .ok_or_else(|| "click target=label needs label".to_string())?
                        .into();
                }
                "at" => {
                    req["x"] = u64_argument(arguments, "x")
                        .ok_or_else(|| "click target=at needs x".to_string())?
                        .into();
                    req["y"] = u64_argument(arguments, "y")
                        .ok_or_else(|| "click target=at needs y".to_string())?
                        .into();
                }
                other => return Err(format!("unknown click target: {other}")),
            }
            req["button"] = u64_argument(arguments, "button")
                .ok_or_else(|| "click needs button".to_string())?
                .into();
        }
        "resize" => {
            req["cols"] = u64_argument(arguments, "cols")
                .ok_or_else(|| "resize needs cols".to_string())?
                .into();
            req["rows"] = u64_argument(arguments, "rows")
                .ok_or_else(|| "resize needs rows".to_string())?
                .into();
        }
        other => return Err(format!("unknown tool: {other}")),
    }
    Ok(req)
}

/// One request, every JSONL reply line collected as `Value`s -- the same
/// exchange `interactive-shell-input` does per invocation.
fn exchange(socket: &Path, request: &Value) -> Result<Vec<Value>, String> {
    let mut stream = connect_in_directory(socket)
        .map_err(|error| format!("connect to {}: {error}", socket.display()))?;
    stream
        .write_all(format!("{request}\n").as_bytes())
        .map_err(|error| error.to_string())?;
    stream
        .shutdown(Shutdown::Write)
        .map_err(|error| error.to_string())?;
    let mut out = String::new();
    stream
        .read_to_string(&mut out)
        .map_err(|error| error.to_string())?;
    out.lines()
        .map(|line| {
            serde_json::from_str(line)
                .map_err(|error| format!("reply line is not JSON: {error}: {line:?}"))
        })
        .collect()
}

// ─────────────────────────────────────────────────────────────────────────────
// start: spawns the real interactive-shell binary as a detached child.
// ─────────────────────────────────────────────────────────────────────────────

/// How long `start` waits for the socket/discovery file to appear before
/// returning `ready: false` rather than failing outright -- the child may
/// legitimately still be starting (a slow command, a cold filesystem cache),
/// and every other tool's own `connect_in_directory` call will simply retry
/// against it later.
const START_READY_BUDGET: Duration = Duration::from_secs(5);
const START_READY_POLL: Duration = Duration::from_millis(20);

fn interactive_shell_binary_path() -> PathBuf {
    let name = if cfg!(windows) {
        "interactive-shell.exe"
    } else {
        "interactive-shell"
    };
    std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|dir| dir.join(name)))
        .filter(|path| path.is_file())
        .unwrap_or_else(|| PathBuf::from(name))
}

fn start_session(arguments: &Value) -> Result<Value, String> {
    let command: Vec<String> = arguments
        .get("command")
        .and_then(Value::as_array)
        .ok_or_else(|| "start needs command (an array of strings)".to_string())?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| "command must be an array of strings".to_string())
        })
        .collect::<Result<_, _>>()?;
    if command.is_empty() {
        return Err("command must not be empty".into());
    }

    let socket_arg = string_argument(arguments, "socket");
    let session_arg = string_argument(arguments, "session");
    let agent_arg = string_argument(arguments, "agent");

    let mut cli_args: Vec<String> = Vec::new();
    if let Some(socket) = &socket_arg {
        cli_args.push("--socket".into());
        cli_args.push(socket.clone());
    } else if let Some(session) = &session_arg {
        cli_args.push("--session".into());
        cli_args.push(session.clone());
    } else if let Some(agent) = &agent_arg {
        cli_args.push("--agent".into());
        cli_args.push(agent.clone());
    } else {
        return Err("start needs socket, session, or agent".into());
    }
    if let Some(cols) = u64_argument(arguments, "cols") {
        cli_args.push("--cols".into());
        cli_args.push(cols.to_string());
    }
    if let Some(rows) = u64_argument(arguments, "rows") {
        cli_args.push("--rows".into());
        cli_args.push(rows.to_string());
    }
    if let Some(idle) = u64_argument(arguments, "idle_timeout_seconds") {
        cli_args.push("--idle-timeout".into());
        cli_args.push(idle.to_string());
    }
    if bool_argument(arguments, "tcp") {
        cli_args.push("--tcp".into());
    }
    cli_args.push("--".into());
    cli_args.extend(command.iter().cloned());

    // What every other tool will resolve to, computed the same way
    // interactive-shell itself does -- so the response can say where the
    // session actually landed even though this call never has to load the
    // session file itself.
    let socket = match &socket_arg {
        Some(socket) => PathBuf::from(socket),
        None => {
            let id = session_identity(session_arg.as_deref(), agent_arg.as_deref())
                .ok_or_else(|| "start needs socket, session, or agent".to_string())?;
            interactive_shell_core::session_socket(&id)?
        }
    };

    let binary = interactive_shell_binary_path();
    let mut child = Command::new(&binary)
        .args(&cli_args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|error| format!("could not start {}: {error}", binary.display()))?;
    let pid = child.id();
    // Detached, but reaped: this adapter's own process can run for a long
    // time and start many sessions, and never calling wait() on an exited
    // child leaves it a zombie until THIS process exits, not until the
    // session's own idle timeout or shutdown -- a slow leak across a long
    // MCP session. A background thread owns the wait() so this call returns
    // immediately without blocking on the session's own lifetime.
    std::thread::spawn(move || {
        let _ = child.wait();
    });

    let deadline = Instant::now() + START_READY_BUDGET;
    let mut ready = false;
    while Instant::now() < deadline {
        if socket.exists() {
            ready = true;
            break;
        }
        std::thread::sleep(START_READY_POLL);
    }

    Ok(json!({
        "started": true,
        "pid": pid,
        "socket": socket.to_string_lossy(),
        "ready": ready,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_reports_server_name() {
        let response = handle(json!({"jsonrpc":"2.0","id":1,"method":"initialize"}));
        assert_eq!(
            response["result"]["serverInfo"]["name"],
            json!("interactive-shell")
        );
    }

    #[test]
    fn tools_list_names_every_routed_tool() {
        let response = handle(json!({"jsonrpc":"2.0","id":1,"method":"tools/list"}));
        let tools = response["result"]["tools"].as_array().unwrap();
        let names: Vec<&str> = tools
            .iter()
            .map(|tool| tool["name"].as_str().unwrap())
            .collect();
        for (name, ..) in routing() {
            assert!(names.contains(name), "tools/list missing {name}");
        }
    }

    #[test]
    fn every_tool_argument_has_a_schema_entry() {
        // tool_argument() panics on an unknown key -- this just has to touch
        // every key TOOL_ARGUMENTS declares, matching chat-mcp's own test.
        for key in TOOL_ARGUMENTS {
            let _ = tool_argument(key);
        }
    }

    #[test]
    fn an_unknown_tool_is_a_clean_error_not_a_panic() {
        let response = call_tool(json!(1), json!({"name":"nonexistent","arguments":{}}));
        assert_eq!(response["result"]["isError"], json!(true));
    }

    #[test]
    fn view_rejects_an_unknown_mode() {
        let error = build_request("view", &json!({"mode":"json"})).unwrap_err();
        assert!(error.contains("json"));
    }

    #[test]
    fn row_ranges_rejects_a_backwards_range() {
        let error = row_ranges_argument(&json!({"row_ranges":["5-3"]})).unwrap_err();
        assert!(error.contains("5-3"));
    }

    #[test]
    fn click_requires_a_recognized_target() {
        let error =
            build_request("click", &json!({"target":"coordinates","button":0})).unwrap_err();
        assert!(error.contains("coordinates"));
    }

    #[test]
    fn resolve_socket_needs_one_of_the_three_selectors() {
        let error = resolve_socket(&json!({})).unwrap_err();
        assert!(error.contains("socket, session, or agent"));
    }
}
