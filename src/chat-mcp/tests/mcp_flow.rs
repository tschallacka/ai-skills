// MODE: DEV
//! End-to-end regressions for the adapter's own wire behaviour, driven the way
//! a harness drives it: raw JSON-RPC over stdio against a real chat server.
//!
//! T90's claims are claims about a live exchange — a channel joined without
//! knowing a port, a multi-line message that survives, a `wait` that returns
//! when a message lands. A schema test cannot show any of them, so they run
//! against a real `chat-server-rs`.
//!
//! The harness is isolated deliberately: agents on a developer machine share a
//! live chat home and a live announce beacon, and a test that joined the real
//! channel would be worse than a failing one. So the adapter gets its own
//! `AI_CHAT_HOME`, a session seeded with the test server's address (so
//! resolution stops at its first rung and never reaches discovery), and an
//! `AI_CHAT_BEACON_PORT` nothing announces on.
//!
//! Nothing here is unix-only -- loopback TCP, a child process per side, JSON
//! over stdio -- so it runs on every platform. (It carried a `cfg(unix)` gate
//! from the day it was written, which quietly left the adapter untested on
//! Windows.)

use serde_json::{json, Value};
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

const SESSION: &str = "t90flow";

struct Harness {
    home: PathBuf,
    port: u16,
    server: Child,
    adapter: Child,
    stdin: ChildStdin,
    /// Every line the adapter writes to stdout, read on a thread of its own so
    /// a test can wait for a notification with a deadline instead of blocking.
    lines: Receiver<String>,
    notices: VecDeque<Value>,
    next_id: u64,
}

/// The directory the workspace build puts every binary in. The adapter, the
/// server and the CLI are siblings there.
fn bin_dir() -> PathBuf {
    Path::new(env!("CARGO_BIN_EXE_chat-mcp"))
        .parent()
        .expect("the adapter binary lives in a directory")
        .to_path_buf()
}

fn scratch(name: &str) -> PathBuf {
    let root = std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let dir = root.join(format!(
        "chat-mcp-flow-{name}-{}-{}",
        std::process::id(),
        Instant::now().elapsed().subsec_nanos()
    ));
    std::fs::create_dir_all(dir.join("channels")).expect("scratch home");
    dir
}

fn free_port() -> u16 {
    let probe = TcpListener::bind("127.0.0.1:0").expect("a free port");
    probe.local_addr().expect("bound address").port()
}

impl Harness {
    /// Start a server and an adapter against a private chat home, or None when
    /// this build has no server beside the adapter (a single-crate test leg).
    fn new(name: &str) -> Option<Harness> {
        let server_bin = bin_dir().join(format!("chat-server-rs{}", std::env::consts::EXE_SUFFIX));
        if !server_bin.is_file() {
            eprintln!(
                "mcp_flow[{name}]: SKIPPED — no chat-server-rs beside the adapter in this \
                 build; `cargo test --workspace` drives the flow"
            );
            return None;
        }
        let home = scratch(name);
        let port = free_port();
        let server = Command::new(&server_bin)
            .arg(port.to_string())
            .env("AI_CHAT_HOME", &home)
            .env("AI_CHAT_BIND", "127.0.0.1")
            // The server reads `CHAT_BEACON_PORT` (no `AI_` prefix); the
            // client reads `AI_CHAT_BEACON_PORT` -- deliberately different
            // names, not a typo (see src/chat-client-rs/tests/resolution.rs,
            // migrated from chat/tests/test-chat-resolution.sh in T145 goal
            // 25, which exercises this exact pairing). Setting only the
            // client-side name here left this test's own server announcing on
            // the real machine's default beacon port (7780) instead of
            // nowhere: latent until a call needed real discovery rather than
            // the pre-seeded session (T143's session/agent-override tests
            // were the first).
            .env("CHAT_BEACON_PORT", (port + 1).to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("chat-server-rs starts");
        wait_for_port(port);
        // Seed the session so resolution takes its first rung: an explicit
        // saved server. Discovery is deliberately never reached.
        let sessions = home.join("sessions");
        std::fs::create_dir_all(&sessions).expect("sessions dir");
        std::fs::write(
            sessions.join(format!("{SESSION}.json")),
            json!({"server": format!("127.0.0.1:{port}"), "nick": "tester", "cursors": {}})
                .to_string(),
        )
        .expect("session file");

        let mut adapter = Command::new(bin_dir().join("chat-mcp"))
            .env("AI_CHAT_HOME", &home)
            .env("CHAT_SESSION_ID", SESSION)
            .env("AI_CHAT_BEACON_PORT", (port + 1).to_string())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("chat-mcp starts");
        let stdin = adapter.stdin.take().expect("adapter stdin");
        let stdout = BufReader::new(adapter.stdout.take().expect("adapter stdout"));
        let (sender, lines) = channel();
        std::thread::spawn(move || {
            for line in stdout.lines().map_while(Result::ok) {
                if sender.send(line).is_err() {
                    break;
                }
            }
        });
        Some(Harness {
            home,
            port,
            server,
            adapter,
            stdin,
            lines,
            notices: VecDeque::new(),
            next_id: 1,
        })
    }

    fn request(&mut self, method: &str, params: Value) -> Value {
        self.next_id += 1;
        let id = self.next_id;
        let line = json!({"jsonrpc":"2.0","id":id,"method":method,"params":params}).to_string();
        writeln!(self.stdin, "{line}").expect("write request");
        self.stdin.flush().expect("flush request");
        self.next_response()
    }

    /// Write a `tools/call` without reading its answer, so two can be in flight
    /// at once; returns the id its answer will carry.
    fn start_call(&mut self, name: &str, arguments: Value) -> u64 {
        self.next_id += 1;
        let id = self.next_id;
        let line = json!({"jsonrpc":"2.0","id":id,"method":"tools/call",
            "params":{"name":name,"arguments":arguments}})
        .to_string();
        writeln!(self.stdin, "{line}").expect("write request");
        self.stdin.flush().expect("flush request");
        id
    }

    /// The next response line, whichever request it answers. A notification
    /// that arrives first is kept for `notice`, not returned as a response.
    fn next_response(&mut self) -> Value {
        loop {
            let line = self
                .lines
                .recv_timeout(Duration::from_secs(60))
                .expect("a response within a minute");
            let message: Value =
                serde_json::from_str(&line).unwrap_or_else(|_| panic!("not JSON: {line}"));
            if message.get("id").is_none() && message.get("method").is_some() {
                self.notices.push_back(message);
                continue;
            }
            return message;
        }
    }

    /// The next `notifications/claude/channel` the adapter pushed, or None
    /// when none arrives within `wait`. Only the wait is bounded: a real push
    /// lands in well under a second.
    fn notice(&mut self, wait: Duration) -> Option<Value> {
        if let Some(notice) = self.notices.pop_front() {
            return Some(notice);
        }
        let deadline = Instant::now() + wait;
        loop {
            let left = deadline.checked_duration_since(Instant::now())?;
            let line = self.lines.recv_timeout(left).ok()?;
            let message: Value =
                serde_json::from_str(&line).unwrap_or_else(|_| panic!("not JSON: {line}"));
            if message.get("id").is_none() && message.get("method").is_some() {
                return Some(message);
            }
            panic!("a response arrived while waiting for a notice: {line}");
        }
    }

    /// A tool call's payload, parsed back out of the text content the MCP
    /// result carries. Panics on an error result, naming it: a tool that
    /// refused is never the thing a caller wanted.
    fn call(&mut self, name: &str, arguments: Value) -> Value {
        let response = self.request("tools/call", json!({"name":name,"arguments":arguments}));
        let result = &response["result"];
        assert!(
            result["isError"] != json!(true),
            "{name} refused: {}",
            result["content"][0]["text"]
        );
        let text = result["content"][0]["text"]
            .as_str()
            .unwrap_or_else(|| panic!("{name} returned no text: {response}"));
        serde_json::from_str(text).unwrap_or_else(|_| panic!("{name} text is not JSON: {text}"))
    }

    /// Post a message as somebody else, through the CLI, so what the adapter
    /// receives is a genuine pushed broadcast from another connection.
    fn other_sends(&self, chan: &str, text: &str) {
        let status = Command::new(bin_dir().join("chat-client-rs"))
            .args([
                "send",
                "--server",
                &format!("127.0.0.1:{}", self.port),
                "--nick",
                "other",
                "--chan",
                chan,
                "--text",
                text,
                "--no-session",
            ])
            .env("AI_CHAT_HOME", &self.home)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("chat-client-rs runs");
        assert!(status.success(), "the other agent's send failed");
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        let _ = self.adapter.kill();
        let _ = self.adapter.wait();
        let _ = self.server.kill();
        let _ = self.server.wait();
        let _ = std::fs::remove_dir_all(&self.home);
    }
}

fn wait_for_port(port: u16) {
    let deadline = Instant::now() + Duration::from_secs(10);
    while Instant::now() < deadline {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("chat-server-rs never accepted on {port}");
}

/// Joining a channel is a tool call: no port, chat home, state directory or
/// subcommand appears in what the caller sends.
#[test]
fn a_channel_is_joined_read_and_posted_to_with_no_port_in_sight() {
    let Some(mut harness) = Harness::new("roundtrip") else {
        return;
    };
    let tools = harness.request("tools/list", json!({}));
    let names: Vec<&str> = tools["result"]["tools"]
        .as_array()
        .expect("a tool list")
        .iter()
        .map(|tool| tool["name"].as_str().unwrap_or_default())
        .collect();
    for expected in ["status", "join", "send", "read", "wait", "who", "channels"] {
        assert!(names.contains(&expected), "{expected} is not advertised");
    }

    let joined = harness.call("join", json!({"channel":"#t90"}));
    assert_eq!(joined["cursor"], json!(0), "an empty channel ends at 0");

    let sent = harness.call("send", json!({"channel":"#t90","text":"first"}));
    assert_eq!(sent["cursor"], json!(1), "the first stored message is id 1");

    // The sender's own message is already accounted for by the cursor, so the
    // delta is what OTHERS said. This is the read an agent actually wants.
    harness.other_sends("#t90", "second");
    let delta = harness.call("read", json!({"channel":"#t90"}));
    let messages = delta["messages"].as_array().expect("messages");
    assert_eq!(messages.len(), 1, "expected one new message: {delta}");
    assert_eq!(messages[0]["nick"], json!("other"));
    assert_eq!(messages[0]["text"], json!("second"));
    assert_eq!(messages[0]["id"], json!(2));

    // And the cursor moved, so the same read again returns nothing rather than
    // the message twice.
    let again = harness.call("read", json!({"channel":"#t90"}));
    assert_eq!(again["messages"].as_array().map(Vec::len), Some(0));
}

/// A join records a cursor of 0 on an empty channel, and 0 is a real cursor —
/// not the absence of one. Read them alike and the first message a brand-new
/// channel ever receives is skipped: the read resolves "no cursor" by asking
/// the server for the current end, which is already past it.
#[test]
fn the_first_message_a_new_channel_receives_is_not_skipped() {
    let Some(mut harness) = Harness::new("firstmessage") else {
        return;
    };
    let joined = harness.call("join", json!({"channel":"#t90f"}));
    assert_eq!(joined["cursor"], json!(0), "an empty channel ends at 0");
    harness.other_sends("#t90f", "the very first thing said here");
    let delta = harness.call("read", json!({"channel":"#t90f"}));
    let messages = delta["messages"].as_array().expect("messages");
    assert_eq!(messages.len(), 1, "the first message was skipped: {delta}");
    assert_eq!(messages[0]["id"], json!(1));
}

/// B266's shape on the adapter's path: a newline in an IRC message is a line
/// terminator, so an unsplit send posts the first line and loses the rest while
/// reporting the whole text as sent.
#[test]
fn a_multi_line_message_keeps_every_line() {
    let Some(mut harness) = Harness::new("multiline") else {
        return;
    };
    harness.call("join", json!({"channel":"#t90m"}));
    let sent = harness.call(
        "send",
        json!({"channel":"#t90m","text":"alpha\nbeta\ngamma"}),
    );
    assert_eq!(sent["cursor"], json!(3), "three lines, three stored rows");
    let history = harness.call("read", json!({"channel":"#t90m","since":0}));
    let texts: Vec<String> = history["messages"]
        .as_array()
        .expect("messages")
        .iter()
        .map(|row| row["text"].as_str().unwrap_or_default().to_string())
        .collect();
    assert_eq!(texts, vec!["alpha", "beta", "gamma"], "{history}");
}

/// `wait` blocks on the held connection, so another agent's message comes back
/// on the push — well inside the timeout, with no fetch in between.
#[test]
fn wait_returns_when_a_message_lands_not_on_the_next_poll() {
    let Some(mut harness) = Harness::new("wait") else {
        return;
    };
    harness.call("join", json!({"channel":"#t90w"}));
    let home = harness.home.clone();
    let port = harness.port;
    let bin = bin_dir().join("chat-client-rs");
    // Sent from another thread a second in, so the wait is genuinely blocked
    // when the message arrives rather than finding it already buffered.
    let sender = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(1));
        let status = Command::new(bin)
            .args([
                "send",
                "--server",
                &format!("127.0.0.1:{port}"),
                "--nick",
                "other",
                "--chan",
                "#t90w",
                "--text",
                "ping @tester",
                "--no-session",
            ])
            .env("AI_CHAT_HOME", &home)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("chat-client-rs runs");
        assert!(status.success());
    });
    let started = Instant::now();
    let woke = harness.call(
        "wait",
        json!({"channel":"#t90w","timeout_seconds":30,"mentions":true}),
    );
    let elapsed = started.elapsed();
    sender.join().expect("the other agent finished");
    assert!(
        woke["timed_out"] != json!(true),
        "wait timed out instead of waking: {woke}"
    );
    assert!(
        elapsed < Duration::from_secs(20),
        "wait took {elapsed:?}, which is a poll rather than a push"
    );
    let messages = woke["messages"].as_array().expect("messages");
    assert!(
        messages
            .iter()
            .any(|row| row["text"] == json!("ping @tester")),
        "the pushed message is missing: {woke}"
    );
}

/// T104: a registered trigger wakes `wait` on a message that mentions
/// nobody at all -- the actual case that cost the most (an instruction
/// addressed to nobody in particular). Same threaded shape as
/// `wait_returns_when_a_message_lands_not_on_the_next_poll`, but the message
/// carries no `@nick` -- only the trigger phrase.
#[test]
fn a_registered_trigger_wakes_wait_on_a_message_with_no_mention_at_all() {
    let Some(mut harness) = Harness::new("trigger") else {
        return;
    };
    harness.call("join", json!({"channel":"#t104"}));
    let added = harness.call("trigger_add", json!({"pattern":"install"}));
    let trigger_id = added["trigger_id"].clone();
    assert_ne!(
        trigger_id,
        Value::Null,
        "trigger_add did not return an id: {added}"
    );

    let home = harness.home.clone();
    let port = harness.port;
    let bin = bin_dir().join("chat-client-rs");
    let sender = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(1));
        let status = Command::new(bin)
            .args([
                "send",
                "--server",
                &format!("127.0.0.1:{port}"),
                "--nick",
                "other",
                "--chan",
                "#t104",
                "--text",
                "q6 write all configs, cleanup too, and install the new build",
                "--no-session",
            ])
            .env("AI_CHAT_HOME", &home)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .expect("chat-client-rs runs");
        assert!(status.success());
    });
    let started = Instant::now();
    let woke = harness.call(
        "wait",
        json!({"channel":"#t104","timeout_seconds":30,"mentions":true}),
    );
    let elapsed = started.elapsed();
    sender.join().expect("the other agent finished");
    assert!(
        woke["timed_out"] != json!(true),
        "wait timed out instead of waking on the trigger: {woke}"
    );
    assert!(
        elapsed < Duration::from_secs(20),
        "wait took {elapsed:?}, which is a poll rather than a push"
    );
    let messages = woke["messages"].as_array().expect("messages");
    assert!(
        messages
            .iter()
            .any(|row| row["text"].as_str().unwrap_or_default().contains("install")),
        "the pushed message is missing: {woke}"
    );

    // A disabled trigger stops firing without losing its definition.
    harness.call(
        "trigger_toggle",
        json!({"trigger_id": trigger_id, "enabled": false}),
    );
    harness.other_sends("#t104", "install again, still nobody mentioned");
    let after_disable = harness.call(
        "wait",
        json!({"channel":"#t104","timeout_seconds":1,"mentions":true}),
    );
    assert_eq!(
        after_disable["timed_out"],
        json!(true),
        "a disabled trigger still woke wait: {after_disable}"
    );

    let listed = harness.call("triggers", json!({}));
    let entries = listed["triggers"].as_array().expect("triggers");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0]["trigger_id"], trigger_id);
    assert_eq!(entries[0]["enabled"], json!(false));

    harness.call("trigger_remove", json!({"trigger_id": trigger_id}));
    let after_remove = harness.call("triggers", json!({}));
    assert_eq!(
        after_remove["triggers"].as_array().map(Vec::len),
        Some(0),
        "trigger_remove left a stale entry: {after_remove}"
    );
}

/// A sender-scoped trigger only fires from that exact nick -- proven end to
/// end, not just at the pure-function level `conn::tests` already covers.
#[test]
fn a_sender_scoped_trigger_ignores_a_matching_message_from_someone_else() {
    let Some(mut harness) = Harness::new("triggerscope") else {
        return;
    };
    harness.call("join", json!({"channel":"#t104b"}));
    harness.call(
        "trigger_add",
        json!({"pattern":"install","sender":"michael"}),
    );
    // "other", not "michael": the pattern matches but the sender does not.
    harness.other_sends("#t104b", "please install this");
    let unmatched = harness.call(
        "wait",
        json!({"channel":"#t104b","timeout_seconds":1,"mentions":true}),
    );
    assert_eq!(
        unmatched["timed_out"],
        json!(true),
        "a trigger scoped to a different sender fired anyway: {unmatched}"
    );
}

/// Presence: who is on the channel right now.
#[test]
fn who_reports_the_members_the_server_knows() {
    let Some(mut harness) = Harness::new("who") else {
        return;
    };
    harness.call("join", json!({"channel":"#t90who"}));
    let members = harness.call("who", json!({"channel":"#t90who"}));
    let list: Vec<String> = members["members"]
        .as_array()
        .expect("members")
        .iter()
        .map(|name| name.as_str().unwrap_or_default().to_string())
        .collect();
    assert!(list.contains(&"tester".to_string()), "{members}");
}

/// `status` answers what an agent would otherwise guess: which server, which
/// nick, which home, and where each channel's reading is up to.
#[test]
fn status_needs_no_server_and_names_what_was_resolved() {
    let Some(mut harness) = Harness::new("status") else {
        return;
    };
    let status = harness.call("status", json!({}));
    assert_eq!(status["nick"], json!("tester"));
    assert_eq!(status["session"], json!(SESSION));
    assert_eq!(status["session_from"], json!("explicit"));
    assert_eq!(status["connection_held"], json!(false), "{status}");
    harness.call("join", json!({"channel":"#t90s"}));
    let status = harness.call("status", json!({}));
    assert_eq!(status["connection_held"], json!(true), "{status}");
}

/// A refusal is a refusal, with the reason in it: the adapter must not let a
/// channel name the server would reject reach the wire, and must not answer a
/// missing argument by connecting first and failing on discovery.
#[test]
fn a_bad_argument_is_refused_by_name() {
    let Some(mut harness) = Harness::new("refusal") else {
        return;
    };
    let response = harness.request(
        "tools/call",
        json!({"name":"send","arguments":{"channel":"#t90r"}}),
    );
    assert_eq!(response["result"]["isError"], json!(true));
    let text = response["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or_default();
    assert!(text.contains("text"), "unexpected refusal: {text}");
}

/// T143: a subagent that declares its own identity gets its own nick and
/// connection, separate from its parent's -- proven by a real `who` on a
/// channel every identity joined, and by a message one identity sent
/// carrying that identity's nick when another connection reads it back.
#[test]
fn distinct_session_overrides_get_their_own_nick_and_hold_separate_connections() {
    let Some(mut harness) = Harness::new("multiplex") else {
        return;
    };
    harness.call("join", json!({"channel":"#t143"}));
    harness.call("join", json!({"channel":"#t143","session":"sub-a"}));
    harness.call("join", json!({"channel":"#t143","session":"sub-b"}));

    let who = harness.call("who", json!({"channel":"#t143"}));
    let members: Vec<String> = who["members"]
        .as_array()
        .expect("members")
        .iter()
        .map(|m| m.as_str().unwrap_or_default().to_string())
        .collect();
    assert!(members.contains(&"tester".to_string()), "{who}");
    assert!(members.contains(&"agent-suba".to_string()), "{who}");
    assert!(members.contains(&"agent-subb".to_string()), "{who}");

    harness.call(
        "send",
        json!({"channel":"#t143","text":"hi from a","session":"sub-a"}),
    );
    let read = harness.call("read", json!({"channel":"#t143"}));
    let senders: Vec<String> = read["messages"]
        .as_array()
        .expect("messages")
        .iter()
        .map(|m| m["nick"].as_str().unwrap_or_default().to_string())
        .collect();
    assert!(
        senders.contains(&"agent-suba".to_string()),
        "the message must be attributed to the sub-a identity's own nick, not the default \
         connection's: {read}"
    );
}

/// `agent` is documented as an alias for `session`; prove it resolves to the
/// exact same identity rather than a second, silently different one.
#[test]
fn the_agent_argument_is_an_alias_for_session() {
    let Some(mut harness) = Harness::new("agent-alias") else {
        return;
    };
    harness.call("join", json!({"channel":"#t143b","agent":"sub-a"}));
    let who = harness.call("who", json!({"channel":"#t143b"}));
    let members: Vec<String> = who["members"]
        .as_array()
        .expect("members")
        .iter()
        .map(|m| m.as_str().unwrap_or_default().to_string())
        .collect();
    assert!(members.contains(&"agent-suba".to_string()), "{who}");
}

/// An empty override is not a declared identity: it must fall back to the
/// adapter's own default connection rather than minting a nick from nothing.
#[test]
fn an_empty_session_argument_falls_back_to_the_default_identity() {
    let Some(mut harness) = Harness::new("empty-session") else {
        return;
    };
    harness.call("join", json!({"channel":"#t143c","session":""}));
    let who = harness.call("who", json!({"channel":"#t143c"}));
    let members: Vec<String> = who["members"]
        .as_array()
        .expect("members")
        .iter()
        .map(|m| m.as_str().unwrap_or_default().to_string())
        .collect();
    assert!(members.contains(&"tester".to_string()), "{who}");
}

/// B363: a blocked `wait` used to hold up every other request to the adapter,
/// because the transport read one request at a time and the connection map's
/// lock was held for the whole call. Two agents that each waited then starved
/// each other. With a 6 s wait in flight, a request from another identity must
/// be answered in well under that, and before the wait.
#[test]
fn a_long_wait_does_not_hold_up_a_request_from_another_identity() {
    let Some(mut harness) = Harness::new("wait-other-identity") else {
        return;
    };
    harness.call("join", json!({"channel":"#b363a"}));
    harness.call("join", json!({"channel":"#b363a","session":"sub-a"}));

    let wait_id = harness.start_call("wait", json!({"channel":"#b363a","timeout_seconds":6}));
    std::thread::sleep(Duration::from_millis(300));
    let started = Instant::now();
    let send_id = harness.start_call(
        "send",
        json!({"channel":"#b363a","text":"while you wait","session":"sub-a"}),
    );

    let first = harness.next_response();
    assert_eq!(
        first["id"],
        json!(send_id),
        "the other identity's send must be answered before the 6 s wait ends: {first}"
    );
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "the send took {:?} with a wait in flight",
        started.elapsed()
    );
    // The waiting identity's own connection is not sent to itself: the wait
    // has to run out its time, and is answered last.
    let second = harness.next_response();
    assert_eq!(second["id"], json!(wait_id), "{second}");
}

/// B363: the same holds within one identity. A `wait` occupied the owner
/// thread, so the identity's own next call queued behind it for the whole
/// timeout; it must instead be served between the wait's ticks.
#[test]
fn a_long_wait_does_not_hold_up_the_same_identitys_next_call() {
    let Some(mut harness) = Harness::new("wait-same-identity") else {
        return;
    };
    harness.call("join", json!({"channel":"#b363b"}));

    let wait_id = harness.start_call("wait", json!({"channel":"#b363b","timeout_seconds":6}));
    std::thread::sleep(Duration::from_millis(300));
    let started = Instant::now();
    let who_id = harness.start_call("who", json!({"channel":"#b363b"}));

    let first = harness.next_response();
    assert_eq!(
        first["id"],
        json!(who_id),
        "the identity's own who must be answered before its 6 s wait ends: {first}"
    );
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "the who took {:?} with a wait in flight",
        started.elapsed()
    );
    let second = harness.next_response();
    assert_eq!(second["id"], json!(wait_id), "{second}");
}

// ---- T150: interrupts and timers, pushed as channel notifications ----------

const NOTICE_WAIT: Duration = Duration::from_secs(10);
const QUIET_WAIT: Duration = Duration::from_millis(1200);

fn content_of(notice: &Value) -> &str {
    notice["params"]["content"].as_str().unwrap_or("")
}

/// A rule pushes a notice for the message it matches and for nothing else, and
/// the notice does not consume what it announced: `read` still returns it.
#[test]
fn a_rule_pushes_a_notice_for_a_matching_message_and_read_still_returns_it() {
    let Some(mut harness) = Harness::new("interrupt-match") else {
        return;
    };
    harness.call("join", json!({"channel":"#flow"}));
    let added = harness.call(
        "interrupt_add",
        json!({"name":"deploys","channels":["#flow"],"from":["@other"],"contains":["deploy"]}),
    );
    assert_eq!(added["state"]["id"], json!(1), "{added}");

    harness.other_sends("#flow", "lunch anyone");
    assert!(
        harness.notice(QUIET_WAIT).is_none(),
        "a message no rule matches must not interrupt"
    );

    harness.other_sends("#flow", "the deploy failed");
    let notice = harness.notice(NOTICE_WAIT).expect("a notice for the match");
    assert_eq!(notice["method"], json!("notifications/claude/channel"));
    assert_eq!(content_of(&notice), "#flow <other> the deploy failed");
    let meta = &notice["params"]["meta"];
    assert_eq!(meta["kind"], json!("message"));
    assert_eq!(meta["channel"], json!("#flow"));
    assert_eq!(meta["from"], json!("other"));
    assert_eq!(meta["rule"], json!("1"));
    assert_eq!(meta["rule_name"], json!("deploys"));

    let read = harness.call("read", json!({"channel":"#flow"}));
    let texts: Vec<&str> = read["messages"]
        .as_array()
        .expect("messages")
        .iter()
        .map(|m| m["text"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(texts, ["lunch anyone", "the deploy failed"]);
}

/// The rules are the agent's to change while it works: modify one and the very
/// next message is judged by the new version; remove it and it stops.
#[test]
fn a_rule_can_be_changed_and_removed_while_the_agent_is_running() {
    let Some(mut harness) = Harness::new("interrupt-modify") else {
        return;
    };
    harness.call("join", json!({"channel":"#flow"}));
    harness.call("interrupt_add", json!({"contains":["alpha"]}));

    harness.other_sends("#flow", "beta");
    assert!(harness.notice(QUIET_WAIT).is_none());

    let updated = harness.call("interrupt_update", json!({"id":1,"contains":["beta"]}));
    assert_eq!(updated["state"]["contains"], json!(["beta"]));
    harness.other_sends("#flow", "beta again");
    let notice = harness
        .notice(NOTICE_WAIT)
        .expect("the modified rule fires");
    assert!(content_of(&notice).ends_with("beta again"), "{notice}");

    harness.call("interrupt_remove", json!({"id":1}));
    harness.other_sends("#flow", "beta once more");
    assert!(
        harness.notice(QUIET_WAIT).is_none(),
        "a removed rule is silent"
    );

    let list = harness.call("interrupt_list", json!({}));
    assert_eq!(list["state"]["rules"], json!([]));
}

/// A snooze holds message notices back without losing the messages, and
/// ending it lets the next one through.
#[test]
fn a_snooze_holds_notices_back_and_ending_it_lets_the_next_one_through() {
    let Some(mut harness) = Harness::new("interrupt-snooze") else {
        return;
    };
    harness.call("join", json!({"channel":"#flow"}));
    harness.call("interrupt_add", json!({}));
    harness.call("interrupt_settings", json!({"snooze_seconds":300}));

    harness.other_sends("#flow", "held back");
    assert!(harness.notice(QUIET_WAIT).is_none());

    let settings = harness.call("interrupt_settings", json!({"snooze_seconds":0}));
    assert_eq!(
        settings["state"]["settings"]["held_back_since_last_notice"],
        json!(1)
    );
    harness.other_sends("#flow", "let through");
    let notice = harness.notice(NOTICE_WAIT).expect("a notice once awake");
    assert_eq!(
        notice["params"]["meta"]["suppressed"],
        json!("1"),
        "{notice}"
    );
    let read = harness.call("read", json!({"channel":"#flow"}));
    assert_eq!(read["messages"].as_array().map(Vec::len), Some(2));
}

/// A timer interrupts by itself, and can be rescheduled and cancelled.
#[test]
fn a_timer_interrupts_and_can_be_rescheduled_and_cancelled() {
    let Some(mut harness) = Harness::new("interrupt-timer") else {
        return;
    };
    let set = harness.call(
        "timer_set",
        json!({"name":"stretch","after_seconds":1,"message":"stand up and stretch"}),
    );
    assert_eq!(set["state"]["id"], json!(1), "{set}");
    let notice = harness.notice(NOTICE_WAIT).expect("the timer fires");
    assert_eq!(content_of(&notice), "stand up and stretch");
    assert_eq!(notice["params"]["meta"]["kind"], json!("timer"));
    assert_eq!(notice["params"]["meta"]["timer_name"], json!("stretch"));

    // Set far off, then pulled in: the reschedule is what makes it fire now.
    harness.call(
        "timer_set",
        json!({"after_seconds":600,"message":"pulled in"}),
    );
    harness.call("timer_update", json!({"id":2,"after_seconds":1}));
    let notice = harness
        .notice(NOTICE_WAIT)
        .expect("the rescheduled timer fires");
    assert_eq!(content_of(&notice), "pulled in");

    harness.call("timer_set", json!({"after_seconds":2,"message":"never"}));
    harness.call("timer_cancel", json!({"id":3}));
    assert!(
        harness.notice(Duration::from_secs(4)).is_none(),
        "a cancelled timer is silent"
    );
}

/// A bad value is refused by name and leaves nothing behind.
#[test]
fn a_bad_interrupt_argument_is_refused_by_name() {
    let Some(mut harness) = Harness::new("interrupt-refuse") else {
        return;
    };
    let refused = harness.request(
        "tools/call",
        json!({"name":"interrupt_add","arguments":{"match":"most"}}),
    );
    assert_eq!(refused["result"]["isError"], json!(true), "{refused}");
    assert!(refused["result"]["content"][0]["text"]
        .as_str()
        .unwrap_or("")
        .contains("match must be"));
    let missing = harness.request(
        "tools/call",
        json!({"name":"interrupt_remove","arguments":{}}),
    );
    assert_eq!(missing["result"]["isError"], json!(true));
    let list = harness.call("interrupt_list", json!({}));
    assert_eq!(list["state"]["rules"], json!([]));
}
