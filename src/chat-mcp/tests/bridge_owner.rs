// MODE: DEV
//! The bridge owns its session's connection: a CLI verb run for the same session
//! is answered by the adapter's held connection, so it never registers under the
//! nick and the server never renames it `<nick>-2`.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{channel, Receiver};
use std::time::{Duration, Instant};

const SESSION: &str = "bridge-owner";
const NICK: &str = "bridge";
const CHAN: &str = "#bridge";

fn bin_dir() -> PathBuf {
    Path::new(env!("CARGO_BIN_EXE_chat-mcp"))
        .parent()
        .expect("the adapter binary lives in a directory")
        .to_path_buf()
}

fn scratch() -> PathBuf {
    let root = std::env::var_os("TMPDIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let dir = root.join(format!(
        "chat-mcp-bridge-owner-{}-{}",
        std::process::id(),
        Instant::now().elapsed().subsec_nanos()
    ));
    std::fs::create_dir_all(&dir).expect("scratch home");
    dir
}

fn free_udp_port() -> u16 {
    let probe = std::net::UdpSocket::bind("0.0.0.0:0").expect("a free UDP port");
    probe.local_addr().expect("bound address").port()
}

fn bound_port(home: &Path) -> u16 {
    let file = home.join("server.port");
    for _ in 0..200 {
        if let Some(port) = std::fs::read_to_string(&file)
            .ok()
            .and_then(|text| text.trim().parse().ok())
        {
            return port;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!(
        "chat-server-rs never recorded its port in {}",
        file.display()
    );
}

fn wait_for_port(port: u16) {
    for _ in 0..200 {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            return;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    panic!("chat-server-rs never accepted on {port}");
}

struct Bridge {
    home: PathBuf,
    port: u16,
    server: Child,
    adapter: Child,
    stdin: ChildStdin,
    lines: Receiver<String>,
    next_id: u64,
}

impl Bridge {
    fn start(home: PathBuf) -> Bridge {
        let beacon_port = free_udp_port();
        let server = Command::new(bin_dir().join("chat-server-rs"))
            .arg("0")
            .env("AI_CHAT_HOME", &home)
            .env("AI_CHAT_BIND", "127.0.0.1")
            .env("CHAT_BEACON_PORT", beacon_port.to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("chat-server-rs starts");
        let port = bound_port(&home);
        wait_for_port(port);
        let sessions = home.join("sessions");
        std::fs::create_dir_all(&sessions).expect("sessions dir");
        std::fs::write(
            sessions.join(format!("{SESSION}.json")),
            json!({"server": format!("127.0.0.1:{port}"), "nick": NICK, "cursors": {}}).to_string(),
        )
        .expect("session file");
        let mut adapter = Command::new(bin_dir().join("chat-mcp"))
            .env("AI_CHAT_HOME", &home)
            .env("CHAT_SESSION_ID", SESSION)
            .env("AI_CHAT_BEACON_PORT", beacon_port.to_string())
            // `interrupt::spool_dir` prefers CLAUDE_CODE_SESSION_ID over the
            // adapter's own session key when it is set, so that it reads it
            // from the harness that is actually driving the agent rather
            // than this test's own synthetic SESSION. Running this suite
            // from inside a real Claude Code session would otherwise spool
            // notices under THAT session's real id instead of the test's
            // own, and the test would read an empty file forever.
            .env_remove("CLAUDE_CODE_SESSION_ID")
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
        Bridge {
            home,
            port,
            server,
            adapter,
            stdin,
            lines,
            next_id: 1,
        }
    }

    /// One tool call on the adapter; panics on an error result.
    fn call(&mut self, name: &str, arguments: Value) -> Value {
        self.next_id += 1;
        let id = self.next_id;
        let line = json!({"jsonrpc":"2.0","id":id,"method":"tools/call",
            "params":{"name":name,"arguments":arguments}})
        .to_string();
        writeln!(self.stdin, "{line}").expect("write request");
        self.stdin.flush().expect("flush request");
        let response: Value = loop {
            let text = self
                .lines
                .recv_timeout(Duration::from_secs(60))
                .expect("a response within a minute");
            let message: Value = serde_json::from_str(&text).expect("JSON from the adapter");
            if message.get("id") == Some(&json!(id)) {
                break message;
            }
        };
        let result = &response["result"];
        assert!(
            result["isError"] != json!(true),
            "{name} refused: {}",
            result["content"][0]["text"]
        );
        let text = result["content"][0]["text"].as_str().expect("tool text");
        serde_json::from_str(text).expect("tool text is JSON")
    }

    /// Run a CLI verb as a separate process for the same session.
    fn cli(&self, args: &[&str]) -> (i32, String) {
        let output = Command::new(bin_dir().join("chat-client-rs"))
            .args(args)
            .args(["--session", SESSION])
            .env("AI_CHAT_HOME", &self.home)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .expect("chat-client-rs runs");
        (
            output.status.code().unwrap_or(-1),
            String::from_utf8_lossy(&output.stdout).into_owned(),
        )
    }

    /// Who the server has in the channel, asked over a fresh connection that
    /// does not go through any session, so it sees the server's own roster.
    fn roster(&self) -> String {
        let output = Command::new(bin_dir().join("chat-client-rs"))
            .args([
                "names",
                "--no-session",
                "--server",
                &format!("127.0.0.1:{}", self.port),
                "--nick",
                "roster-probe",
                "--chan",
                CHAN,
            ])
            .env("AI_CHAT_HOME", &self.home)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .expect("chat-client-rs runs");
        String::from_utf8_lossy(&output.stdout).into_owned()
    }
}

impl Drop for Bridge {
    fn drop(&mut self) {
        let _ = self.adapter.kill();
        let _ = self.adapter.wait();
        let _ = self.server.kill();
        let _ = self.server.wait();
        let _ = std::fs::remove_dir_all(&self.home);
    }
}

/// This identity's interrupt spool file: `chat-mcp`'s own `state_dir()` is
/// `AI_CHAT_HOME` (same resolution as `channels_home`), and no
/// `CLAUDE_CODE_SESSION_ID` is set in this harness, so `interrupt::spool_dir`
/// keys the directory by `SESSION` itself.
fn spool_file(home: &Path) -> PathBuf {
    home.join("interrupts")
        .join(SESSION)
        .join(format!("{SESSION}.log"))
}

/// Send a message as a nick the bridge never registered under, over a fresh
/// connection with no session of its own -- the same pattern `Bridge::roster`
/// uses. A message from the bridge's own nick is never pushed back to it
/// (`Owner::take_push`'s own-message check), so a real external sender is the
/// only way to prove the always-on background loop, not just a synchronous
/// `wait` answer.
fn send_as(home: &Path, port: u16, nick: &str, text: &str) {
    let status = Command::new(bin_dir().join("chat-client-rs"))
        .args([
            "send",
            "--no-session",
            "--server",
            &format!("127.0.0.1:{port}"),
            "--nick",
            nick,
            "--chan",
            CHAN,
            "--text",
            text,
        ])
        .env("AI_CHAT_HOME", home)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("chat-client-rs send runs");
    assert!(status.success(), "send as {nick} must succeed");
}

#[test]
fn a_cli_send_and_names_are_answered_by_the_bridge_and_no_second_nick_appears() {
    // The server and the CLI are sibling binaries of the adapter. A per-crate
    // `cargo test -p chat-mcp` builds only the adapter, so the flow is skipped
    // there, the way mcp_flow.rs skips it; the workspace-wide run drives it.
    for sibling in ["chat-server-rs", "chat-client-rs"] {
        if !bin_dir()
            .join(format!("{sibling}{}", std::env::consts::EXE_SUFFIX))
            .is_file()
        {
            eprintln!("bridge_owner: SKIPPED — no {sibling} beside the adapter in this build");
            return;
        }
    }
    let home = scratch();
    let mut bridge = Bridge::start(home);
    // The first tool call on the identity opens its held connection.
    let joined = bridge.call("join", json!({"channel": CHAN}));
    assert!(joined.is_object(), "join answered: {joined}");
    assert!(
        bridge
            .home
            .join("owners")
            .join(format!("{SESSION}.json"))
            .exists(),
        "the bridge recorded itself as the session's owner"
    );

    let (code, out) = bridge.cli(&["send", "--chan", CHAN, "--text", "from the cli"]);
    assert_eq!(code, 0, "the forwarded send succeeds: {out}");
    assert!(
        out.contains(&format!(
            ":{NICK}!{NICK}@localhost PRIVMSG {CHAN} :from the cli"
        )),
        "the send is reported as the bridge's own: {out}"
    );

    let (code, out) = bridge.cli(&["names", "--chan", CHAN]);
    assert_eq!(code, 0, "the forwarded names succeeds: {out}");
    assert!(out.contains(NICK), "the bridge is a member: {out}");
    assert!(
        !out.contains(&format!("{NICK}-2")),
        "no suffixed nick: {out}"
    );

    let roster = bridge.roster();
    assert!(
        roster.contains(NICK),
        "the server still has the bridge: {roster}"
    );
    assert!(
        !roster.contains(&format!("{NICK}-2")),
        "the server never renamed a second connection: {roster}"
    );
}

#[test]
fn a_forgotten_wait_still_reaches_the_agent_through_its_own_auto_registered_rule() {
    for sibling in ["chat-server-rs", "chat-client-rs"] {
        if !bin_dir()
            .join(format!("{sibling}{}", std::env::consts::EXE_SUFFIX))
            .is_file()
        {
            eprintln!("bridge_owner: SKIPPED — no {sibling} beside the adapter in this build");
            return;
        }
    }
    let home = scratch();
    let mut bridge = Bridge::start(home);
    bridge.call("join", json!({"channel": CHAN}));

    // Nothing has been sent yet: this `wait` times out, but it must still
    // register the standing rule and say so -- the point is that the rule is
    // armed from the very first `wait`, not only after one has already fired.
    let waited = bridge.call("wait", json!({"channel": CHAN, "timeout_seconds": 1}));
    assert_eq!(waited["timed_out"], json!(true), "{waited}");
    let note = waited["note"].as_str().unwrap_or_default();
    assert!(
        note.contains("registered standing interrupt rule"),
        "the first wait on a scope must say so: {note}"
    );

    let spool = spool_file(&bridge.home);
    assert!(
        !spool.exists(),
        "no notice must exist before anything was sent: {}",
        spool.display()
    );

    // No second `wait` call here -- this is exactly the failure mode being
    // fixed: an agent that forgot to re-arm. The message still has to reach
    // it, through the standing rule the first `wait` registered.
    send_as(
        &bridge.home,
        bridge.port,
        "someone-else",
        "did you see this",
    );

    let deadline = Instant::now() + Duration::from_secs(5);
    let mut spooled = String::new();
    while Instant::now() < deadline {
        if let Ok(text) = std::fs::read_to_string(&spool) {
            spooled = text;
            if !spooled.is_empty() {
                break;
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        spooled.contains("did you see this") && spooled.contains("someone-else"),
        "the forgotten wait's own standing rule must have delivered the message: {spooled:?}"
    );

    // The rule itself, read back, confirms it is the auto-registered one and
    // that it is the thing that actually fired.
    let list = bridge.call("interrupt_list", json!({}));
    let rules = list["state"]["rules"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    let auto_rule = rules
        .iter()
        .find(|rule| rule["auto_wait"] == json!(true))
        .unwrap_or_else(|| panic!("no auto_wait rule in {list}"));
    assert!(
        auto_rule["fired"].as_u64().unwrap_or(0) >= 1,
        "the auto-registered rule itself must show it fired: {auto_rule}"
    );

    // A second `wait` on the same scope must not pile up a duplicate rule.
    bridge.call("wait", json!({"channel": CHAN, "timeout_seconds": 1}));
    let list = bridge.call("interrupt_list", json!({}));
    let count = list["state"]["rules"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter(|rule| rule["auto_wait"] == json!(true))
        .count();
    assert_eq!(
        count, 1,
        "a repeated wait on the same scope must stay idempotent: {list}"
    );
}
