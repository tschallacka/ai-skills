// MODE: DEV
// PACKAGE: PROD
//! MCP adapter for the chat client: channels as typed tool calls.
//!
//! T90, against B124. Discovery, the chat home, the session and the TOFU pin
//! are the adapter's business: no schema here carries a port, a path or an
//! insecure escape, so none of them is a model's to choose. What the tools
//! offer instead is what an agent means — join, send, read since my cursor,
//! wait for the next message, who is here.
//!
//! `wait` is what makes this more than a wrapper: the connection is held for
//! the life of the process (see `conn`), so a message is delivered when it
//! lands rather than on the next poll.

pub mod conn;

use conn::{Answer, Failure, Held, Op};
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

/// The longest a `wait` may block. Long enough to be worth holding, short
/// enough that a client's own request timeout is not what ends it.
const WAIT_MAX_SECONDS: u64 = 300;
const WAIT_DEFAULT_SECONDS: u64 = 60;

pub fn handle(message: Value) -> Value {
    let id = message.get("id").cloned().unwrap_or(Value::Null);
    let method = message.get("method").and_then(Value::as_str).unwrap_or("");
    match method {
        "initialize" => json!({"jsonrpc":"2.0","id":id,"result":{
            "protocolVersion":"2025-06-18",
            "capabilities":{"tools":{}},
            "serverInfo":{"name":"chat","version":"0.1.0"}}}),
        "notifications/initialized" => Value::Null,
        "tools/list" => json!({"jsonrpc":"2.0","id":id,"result":{"tools": tool_definitions()}}),
        "tools/call" => call_tool(id, message.get("params").cloned().unwrap_or_default()),
        _ => json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"method not found"}}),
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// The advertised schema. One const declares every argument a tool may carry,
// one exhaustive match describes each, and `routing` names which tool takes
// which. Binding the three together is what keeps an argument the adapter
// reads from being one a schema-following client may not send (the editor
// adapter's B195, B197, B211 and B217 were all that one shape).
// ─────────────────────────────────────────────────────────────────────────────

/// Every argument any tool accepts. The adapter consumes all of them itself:
/// unlike the editor's adapter there is no payload passed through to a server,
/// so this list is the whole vocabulary a client may send.
pub const TOOL_ARGUMENTS: &[&str] = &[
    "channel",
    "mentions",
    "since",
    "text",
    "timeout_seconds",
    "wait_seconds",
    "pattern",
    "sender",
    "trigger_id",
    "enabled",
    "session",
    "agent",
];

/// The advertised schema for one `TOOL_ARGUMENTS` key. Exhaustive on purpose:
/// adding a key to the const without describing it here panics the schema
/// test rather than shipping an argument with no description.
fn tool_argument(key: &str) -> Value {
    match key {
        "channel" => {
            json!({"type":"string","description":"Channel name, including the leading '#': lowercase letters, digits, '_' and '-', at most 32 characters after the '#'."})
        }
        "mentions" => {
            json!({"type":"boolean","description":"Only messages that mention your nick as @nick. A mention-filtered read deliberately does NOT move your cursor, so the messages it skipped are still unread."})
        }
        "since" => {
            json!({"type":"integer","description":"Return messages after this message id instead of after your saved cursor. 0 asks for the whole backlog. On join, seeds the cursor to this id."})
        }
        "text" => {
            json!({"type":"string","description":"The message. Newlines are kept: the text is split into one wire line per line, and long lines are split again at the protocol's own limit, so nothing is silently truncated."})
        }
        "timeout_seconds" => {
            json!({"type":"integer","description":"How long wait may block before answering that nothing arrived. Default 60, maximum 300."})
        }
        "wait_seconds" => {
            json!({"type":"integer","description":"How long discover listens for announce beacons. Default 3."})
        }
        "pattern" => {
            json!({"type":"string","description":"Text to wake on, matched as a substring of the message (case-insensitive) by default. Add your own '*' (any run of characters) or '?' (exactly one) inside it for finer control -- these are the only two wildcards; anything else is matched literally, never as a regex."})
        }
        "sender" => {
            json!({"type":"string","description":"Only a message from this exact nick can fire the trigger. Omit to match a message from anyone."})
        }
        "trigger_id" => {
            json!({"type":"integer","description":"The id trigger_add returned, or one read back from triggers."})
        }
        "enabled" => {
            json!({"type":"boolean","description":"The trigger's new state: true wakes wait/read on it again, false leaves the definition in place but stops it firing."})
        }
        "session" => {
            json!({"type":"string","description":"This agent's own identity, if you have one (e.g. the AGENT_ID a SubagentStart hook gave you). Keeps your nick, cursors and held connection separate from your parent's and from any sibling subagent -- omit it and every call shares one process-wide identity instead."})
        }
        "agent" => {
            json!({"type":"string","description":"Same as session; either name works, session wins if both are given."})
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

/// Every tool: name, description, the arguments it takes, and which of those
/// are required.
fn routing() -> &'static [ToolSpec] {
    &[
        (
            "status",
            "Where this agent stands: the chat server that was resolved and how, the nick it registers as, the state directory, and the per-channel cursors. Needs no arguments and starts nothing.",
            &[],
            &[],
        ),
        (
            "discover",
            "Which chat servers are announcing themselves on the local network, from the UDP beacon. Use it to see whether a server is already running BEFORE starting one: a second server on another port splits the channel, and every agent then talks past the others.",
            &["wait_seconds"],
            &[],
        ),
        (
            "channels",
            "The channels that exist, from the shared message store. A channel exists once its first message is stored.",
            &[],
            &[],
        ),
        (
            "join",
            "Join a channel and start receiving its messages on this connection. Seeds your cursor to the channel's current end, so the first read returns what arrives next rather than the whole backlog; pass since to start further back.",
            &["channel", "since", "session", "agent"],
            &["channel"],
        ),
        (
            "leave",
            "Leave a channel: stop receiving it and forget its cursor, so a later join starts at the end again.",
            &["channel", "session", "agent"],
            &["channel"],
        ),
        (
            "send",
            "Post a message to a channel, and report the id the server stored it as. Multi-line text is kept whole.",
            &["channel", "text", "session", "agent"],
            &["channel", "text"],
        ),
        (
            "read",
            "Every message stored after your cursor, each with its id, and advance the cursor. With no cursor and no since, this returns nothing and records where reading starts rather than dumping the channel's history.",
            &["channel", "since", "mentions", "session", "agent"],
            &["channel"],
        ),
        (
            "wait",
            "Block until a message arrives, then return it as read would. This is what the connection is held for: the message is delivered when it lands, not on a later poll. Answers timed_out rather than failing when nothing arrives.",
            &["channel", "mentions", "timeout_seconds", "session", "agent"],
            &[],
        ),
        (
            "who",
            "Which nicks are in a channel right now, from the server's own membership list.",
            &["channel", "session", "agent"],
            &["channel"],
        ),
        (
            "trigger_add",
            "Register a content-based wake condition for wait/read (mentions mode): any message matching pattern wakes you, in addition to your own @nick mention -- the nick stays a trigger, it stops being the only one. Returns the trigger_id needed to remove or toggle it later.",
            &["pattern", "sender", "session", "agent"],
            &["pattern"],
        ),
        (
            "trigger_remove",
            "Permanently deregister a trigger. Refused by name if trigger_id does not exist (already removed, or never registered by this connection).",
            &["trigger_id", "session", "agent"],
            &["trigger_id"],
        ),
        (
            "trigger_toggle",
            "Enable or disable a trigger without losing its definition, so it can be re-enabled later without calling trigger_add again.",
            &["trigger_id", "enabled", "session", "agent"],
            &["trigger_id", "enabled"],
        ),
        (
            "triggers",
            "Every trigger this connection holds, enabled or not, with its pattern, sender scope and id -- read this back rather than tracking ids yourself.",
            &["session", "agent"],
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
    let result = match name {
        "status" => Ok(status()),
        "discover" => Ok(discover(
            u64_argument(&arguments, "wait_seconds").unwrap_or(3),
        )),
        "channels" => Ok(channels()),
        "join" | "leave" | "send" | "read" | "wait" | "who" | "trigger_add" | "trigger_remove"
        | "trigger_toggle" | "triggers" => connected_tool(name, &arguments),
        other => Err(format!(
            "unknown tool: {}. The tools are status, discover, channels, join, leave, send, read, \
             wait, who, trigger_add, trigger_remove, trigger_toggle and triggers.",
            other
        )),
    };
    match result {
        Ok(value) => json!({"jsonrpc":"2.0","id":id,"result":{"content":[
            {"type":"text","text": serde_json::to_string_pretty(&value).unwrap_or_default()}]}}),
        Err(message) => tool_error(id, &message),
    }
}

fn tool_error(id: Value, message: &str) -> Value {
    json!({"jsonrpc":"2.0","id":id,"result":{"isError":true,"content":[{"type":"text","text":message}]}})
}

/// The tools that need the held connection. Each one names its own required
/// arguments through `routing`, so a missing one is refused here by name
/// rather than reaching the wire as an empty string.
fn connected_tool(name: &str, arguments: &Value) -> Result<Value, String> {
    for key in required_arguments(name) {
        if arguments.get(key).is_none() {
            return Err(format!("{} needs {}", name, key));
        }
    }
    let session_key = resolved_session_key(arguments);
    let chan = string_argument(arguments, "channel");
    if let Some(chan) = chan.as_deref() {
        if !chat_client_rs::valid_chan(chan) {
            return Err(format!(
                "not a channel name: {}. A channel is '#' then lowercase letters, digits, '_' or '-'.",
                chan
            ));
        }
    }
    let mentions = arguments
        .get("mentions")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let since = u64_argument(arguments, "since");
    let op = match name {
        "join" => Op::Join {
            chan: chan.clone().unwrap_or_default(),
            since,
        },
        "leave" => Op::Leave {
            chan: chan.clone().unwrap_or_default(),
        },
        "send" => Op::Send {
            chan: chan.clone().unwrap_or_default(),
            text: string_argument(arguments, "text").unwrap_or_default(),
        },
        "read" => Op::Read {
            chan: chan.clone().unwrap_or_default(),
            since,
            mentions,
        },
        "wait" => Op::Wait {
            chan: chan.clone(),
            mentions,
            timeout: Duration::from_secs(
                u64_argument(arguments, "timeout_seconds")
                    .unwrap_or(WAIT_DEFAULT_SECONDS)
                    .clamp(1, WAIT_MAX_SECONDS),
            ),
        },
        "who" => Op::Who {
            chan: chan.clone().unwrap_or_default(),
        },
        "trigger_add" => Op::TriggerAdd {
            pattern: string_argument(arguments, "pattern").unwrap_or_default(),
            sender: string_argument(arguments, "sender"),
        },
        "trigger_remove" => Op::TriggerRemove {
            id: trigger_id_argument(arguments)?,
        },
        "trigger_toggle" => Op::TriggerToggle {
            id: trigger_id_argument(arguments)?,
            enabled: arguments
                .get("enabled")
                .and_then(Value::as_bool)
                .ok_or("trigger_toggle needs enabled to be true or false")?,
        },
        "triggers" => Op::Triggers,
        other => return Err(format!("unroutable tool: {}", other)),
    };
    let answer = with_connection(&session_key, |held| held.submit(op.clone()))?;
    Ok(answer_value(name, chan.as_deref(), answer))
}

fn required_arguments(name: &str) -> &'static [&'static str] {
    routing()
        .iter()
        .find(|(tool, ..)| *tool == name)
        .map(|(_, _, _, required)| *required)
        .unwrap_or(&[])
}

fn answer_value(tool: &str, chan: Option<&str>, answer: Answer) -> Value {
    let rows: Vec<Value> = answer
        .rows
        .iter()
        .map(|row| json!({"id":row.id,"at":row.ts,"nick":row.nick,"text":row.text}))
        .collect();
    let mut out = Map::new();
    out.insert("tool".into(), json!(tool));
    if let Some(chan) = chan {
        out.insert("channel".into(), json!(chan));
    }
    match tool {
        "who" => {
            out.insert("members".into(), json!(answer.members));
        }
        "trigger_add" => {
            out.insert("trigger_id".into(), json!(answer.trigger_id));
        }
        "trigger_remove" | "trigger_toggle" => {}
        "triggers" => {
            let triggers: Vec<Value> = answer
                .triggers
                .iter()
                .map(|t| {
                    json!({"trigger_id":t.id,"pattern":t.pattern,"sender":t.sender,"enabled":t.enabled})
                })
                .collect();
            out.insert("triggers".into(), Value::Array(triggers));
        }
        _ => {
            out.insert("messages".into(), Value::Array(rows));
            out.insert("cursor".into(), json!(answer.cursor));
        }
    }
    if answer.timed_out {
        out.insert("timed_out".into(), json!(true));
    }
    if let Some(note) = answer.note {
        out.insert("note".into(), json!(note));
    }
    Value::Object(out)
}

fn string_argument(arguments: &Value, key: &str) -> Option<String> {
    arguments
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
}

/// An integer argument, accepting the JSON number a schema-following client
/// sends and the string a hand-written one sometimes does.
fn u64_argument(arguments: &Value, key: &str) -> Option<u64> {
    match arguments.get(key) {
        Some(Value::Number(number)) => number.as_u64(),
        Some(Value::String(text)) => text.trim().parse().ok(),
        _ => None,
    }
}

/// `trigger_id` is required on both tools that take it (checked above by
/// `required_arguments`), so a present-but-unparseable value is refused by
/// name here rather than silently defaulting to some other trigger.
fn trigger_id_argument(arguments: &Value) -> Result<u64, String> {
    u64_argument(arguments, "trigger_id")
        .ok_or_else(|| "trigger_id must be a whole number".to_string())
}

// ─────────────────────────────────────────────────────────────────────────────
// Resolution: the part a model no longer has to do.
// ─────────────────────────────────────────────────────────────────────────────

/// T143: a subagent sharing its parent's Claude Code session is otherwise
/// indistinguishable from it (B303) -- the agent-identity-plugin (T122)
/// hands a subagent its own id specifically so it can declare itself here.
/// One process now holds a connection PER resolved key, not one connection
/// for its whole life: two agents naming different ids get their own nick,
/// cursors and server connection, keyed by exactly what `save_session_with_key`/
/// `Session::load_with_key` already key their on-disk state by.
fn held() -> &'static Mutex<HashMap<String, Held>> {
    static HELD: OnceLock<Mutex<HashMap<String, Held>>> = OnceLock::new();
    HELD.get_or_init(|| Mutex::new(HashMap::new()))
}

/// Run one operation against the connection held for `session_key`, opening
/// it first if this is the first call under that key. A connection the
/// server has closed is discarded and reopened once, so a restarted server
/// costs one retry rather than a dead adapter -- unchanged from the
/// single-connection version, just scoped to one map entry instead of the
/// whole process.
fn with_connection<F>(session_key: &str, operation: F) -> Result<Answer, String>
where
    F: Fn(&Held) -> Result<Answer, Failure>,
{
    let mut map = held()
        .lock()
        .map_err(|_| "the connection lock is poisoned; restart the adapter".to_string())?;
    for attempt in 0..2 {
        if !map.contains_key(session_key) {
            map.insert(session_key.to_string(), open_connection(session_key)?);
        }
        let conn = map.get(session_key).expect("just opened");
        match operation(conn) {
            Ok(answer) => return Ok(answer),
            // The link is gone: drop it and open a new one once, so a restarted
            // server costs a retry rather than a dead adapter.
            Err(failure) if failure.dead => {
                map.remove(session_key);
                if attempt == 1 {
                    return Err(failure.message);
                }
            }
            // The operation itself failed. The connection is fine and keeping
            // it is the point of holding one.
            Err(failure) => return Err(failure.message),
        }
    }
    Err("the connection could not be established".to_string())
}

fn state_dir() -> PathBuf {
    chat_client_rs::client_state_dir(&[])
}

/// The identity a connected-tool call resolves to: an explicit `session`/
/// `agent` argument (`session` winning if both are given) if the caller
/// declared one, else this process's own default identity -- exactly what
/// every call used before T143, unchanged for a caller that never declares
/// one. `resolve_session_key`'s own explicit rung (chat_client_rs) always
/// wins over env/worktree, so env and worktree_root are never consulted for
/// a declared id and are passed as `None` rather than computed for nothing.
fn resolved_session_key(arguments: &Value) -> String {
    let explicit =
        string_argument(arguments, "session").or_else(|| string_argument(arguments, "agent"));
    match explicit.as_deref().filter(|s| !s.is_empty()) {
        Some(id) => chat_client_rs::resolve_session_key(Some(id), &|_| None, None, None).0,
        None => chat_client_rs::session_key().0.clone(),
    }
}

/// The server and nick this agent uses under `session_key`, resolved the way
/// the client resolves them: an explicit saved session first, then a live
/// server from the cache, then the announce beacon.
fn resolve(session_key: &str) -> Result<(String, String), String> {
    let dir = state_dir();
    let (session_server, nick, _) =
        chat_client_rs::apply_session_with_key("", "", &dir, false, session_key);
    let server = chat_client_rs::resolve_server("", &session_server, &dir, false);
    if server.is_empty() {
        return Err(format!(
            "no chat server found: nothing saved, nothing cached, and no announce beacon on UDP {} within 3s. \
             Start ONE server (the chat skill's chat-server-rs) and let its beacon be how clients find it — \
             a second server on another port splits the channel.",
            chat_client_rs::DEFAULT_BEACON_PORT
        ));
    }
    Ok((server, resolved_nick(nick, session_key)))
}

/// The nick to register as. A saved one wins; otherwise one is minted from
/// the resolved session key and saved, so an agent with no setup at all
/// still has a stable identity across calls rather than a fresh one each
/// time.
fn resolved_nick(saved: String, session_key: &str) -> String {
    if !saved.is_empty() {
        return saved;
    }
    let short: String = session_key
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .take(10)
        .collect();
    format!(
        "agent-{}",
        if short.is_empty() {
            "mcp".into()
        } else {
            short
        }
    )
}

fn open_connection(session_key: &str) -> Result<Held, String> {
    let (server, nick) = resolve(session_key)?;
    let dir = state_dir();
    let held = Held::open(&server, &nick, &dir)?;
    chat_client_rs::save_session_with_key(&dir, session_key, &server, &nick);
    Ok(held)
}

// ─────────────────────────────────────────────────────────────────────────────
// The tools that need no connection.
// ─────────────────────────────────────────────────────────────────────────────

fn status() -> Value {
    let dir = state_dir();
    let (key, source) = chat_client_rs::session_key();
    let session = chat_client_rs::Session::load(&dir);
    let cursors: Map<String, Value> = session
        .cursors
        .iter()
        .map(|(chan, id)| (chan.clone(), json!(id)))
        .collect();
    let connected = held()
        .lock()
        .map(|map| map.contains_key(key.as_str()))
        .unwrap_or(false);
    json!({
        "tool": "status",
        "server": session.server,
        "nick": if session.nick.is_empty() { resolved_nick(String::new(), key) } else { session.nick.clone() },
        "nick_is_saved": !session.nick.is_empty(),
        "session": key,
        "session_from": source.as_str(),
        "state_dir": dir.display().to_string(),
        "channel_store": chat_client_rs::channels_home().display().to_string(),
        "connection_held": connected,
        "cursors": Value::Object(cursors),
    })
}

fn discover(wait_seconds: u64) -> Value {
    let port = std::env::var("AI_CHAT_BEACON_PORT")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(chat_client_rs::DEFAULT_BEACON_PORT);
    let found = chat_client_rs::discover_candidates(port, wait_seconds.clamp(1, 30));
    json!({
        "tool": "discover",
        "beacon_port": port,
        "servers": found,
        "note": if found.is_empty() {
            "no server announced itself; start ONE server rather than assuming a port"
        } else {
            "one of these is already serving; join it instead of starting another"
        },
    })
}

fn channels() -> Value {
    let home = chat_client_rs::channels_home();
    let mut names: Vec<String> = Vec::new();
    if let Ok(entries) = std::fs::read_dir(home.join("channels")) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if let Some(chan) = name.strip_suffix(".log") {
                if chat_client_rs::valid_chan(chan) {
                    names.push(chan.to_string());
                }
            }
        }
    }
    names.sort();
    json!({"tool":"channels","channels":names,"store":home.display().to_string()})
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tools() -> Vec<Value> {
        tool_definitions()
    }

    fn properties(tool: &Value) -> Map<String, Value> {
        tool.pointer("/inputSchema/properties")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default()
    }

    /// A tool advertising an empty schema makes every argument it reads
    /// unsendable by a client that follows the schema (B195).
    #[test]
    fn every_tool_that_takes_arguments_advertises_them() {
        for tool in tools() {
            let name = tool["name"].as_str().unwrap_or("?");
            let declared = properties(&tool);
            let expected = routing()
                .iter()
                .find(|(candidate, ..)| *candidate == name)
                .map(|(_, _, arguments, _)| *arguments)
                .expect("every advertised tool is routed");
            assert_eq!(
                declared.len(),
                expected.len(),
                "{name} advertises {declared:?} for arguments {expected:?}"
            );
            for key in expected {
                assert!(declared.contains_key(*key), "{name} does not declare {key}");
            }
        }
    }

    #[test]
    fn every_argument_a_tool_takes_is_in_the_const_and_has_a_schema() {
        for (name, _, arguments, _) in routing() {
            for key in arguments.iter() {
                assert!(
                    TOOL_ARGUMENTS.contains(key),
                    "{name} takes {key}, which TOOL_ARGUMENTS does not declare"
                );
                // Panics through `unreachable!` if the match is not exhaustive
                // over the const.
                let schema = tool_argument(key);
                assert!(
                    schema.get("description").and_then(Value::as_str).is_some(),
                    "{key} has no description"
                );
            }
        }
    }

    #[test]
    fn every_declared_argument_is_used_by_some_tool() {
        for key in TOOL_ARGUMENTS {
            assert!(
                routing()
                    .iter()
                    .any(|(_, _, arguments, _)| arguments.contains(key)),
                "TOOL_ARGUMENTS declares {key} and no tool takes it"
            );
        }
    }

    #[test]
    fn every_required_argument_is_also_declared() {
        for tool in tools() {
            let name = tool["name"].as_str().unwrap_or("?");
            let declared = properties(&tool);
            for key in tool["inputSchema"]["required"]
                .as_array()
                .unwrap_or(&vec![])
            {
                let key = key.as_str().unwrap_or_default();
                assert!(
                    declared.contains_key(key),
                    "{name} requires {key} and does not declare it"
                );
            }
        }
    }

    /// No schema may offer a port, a state directory, a server address or an
    /// insecure escape. Resolving those is the adapter's job (B124).
    #[test]
    fn no_tool_offers_a_port_a_path_or_an_insecure_escape() {
        for key in TOOL_ARGUMENTS {
            for forbidden in ["port", "server", "insecure", "state", "host"] {
                assert!(
                    !key.contains(forbidden),
                    "{key} lets a caller choose {forbidden}"
                );
            }
        }
    }

    #[test]
    fn every_tool_refuses_arguments_it_does_not_declare() {
        for tool in tools() {
            let name = tool["name"].as_str().unwrap_or("?");
            assert_eq!(
                tool["inputSchema"]["additionalProperties"],
                json!(false),
                "{name} accepts undeclared arguments"
            );
        }
    }

    #[test]
    fn the_required_arguments_are_refused_by_name_before_any_connection() {
        // No server is reachable in a unit test, so a tool that skipped this
        // check would fail on discovery instead — which is the wrong error and
        // the slow one.
        let error = connected_tool("send", &json!({"channel":"#x"})).unwrap_err();
        assert!(error.contains("text"), "unexpected error: {error}");
        let error = connected_tool("read", &json!({})).unwrap_err();
        assert!(error.contains("channel"), "unexpected error: {error}");
    }

    #[test]
    fn a_channel_name_the_server_would_refuse_is_refused_here() {
        let error = connected_tool("read", &json!({"channel":"../../etc/passwd"})).unwrap_err();
        assert!(error.contains("not a channel name"), "unexpected: {error}");
    }

    #[test]
    fn an_unknown_method_is_a_jsonrpc_error() {
        let response = handle(json!({"jsonrpc":"2.0","id":1,"method":"nope"}));
        assert_eq!(response["error"]["code"], json!(-32601));
    }

    #[test]
    fn initialize_advertises_tools_and_nothing_else() {
        let response = handle(json!({"jsonrpc":"2.0","id":1,"method":"initialize"}));
        assert_eq!(response["result"]["serverInfo"]["name"], json!("chat"));
        assert!(response["result"]["capabilities"]["tools"].is_object());
    }
}
