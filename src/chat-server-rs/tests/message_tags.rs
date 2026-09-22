// MODE: DEV
//! B157/T131/T135: real IRCv3 CAP negotiation and the message-tags msgid it
//! carries, plus a live `tail`'s cursor advancing from that real msgid tag
//! rather than a once-a-second poll (T145 goal 25, W133).
//!
//! Kept in ONE file per this step's own split-flag (AR-108, cycle 44):
//! CAP negotiation verifiably exercises chat-server-rs only (a raw TLS probe;
//! chat-client-rs does not yet speak CAP), while the tail-cursor half
//! verifiably exercises chat-client-rs's own session-cursor/tail behavior --
//! both trace one coherent B157 IRCv3 feature lifecycle end to end (msgid
//! generation via CAP negotiation, then consumption via tail's cursor).
//! chat-client-rs's own binary is spawned via `support::resolve_workspace_binary`
//! (a real subprocess in the shared workspace target directory), never via
//! `env!("CARGO_BIN_EXE_chat-client-rs")`, which does not resolve across
//! crates in this workspace (AR-112, cycle 45).

mod support;

use std::io::{Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use support::{spawn_server, ChildGuard};

/// Ceiling for a raw TLS connection's openssl handshake plus the server's
/// reply; wait_for returns as soon as the needle appears, so raising this
/// only helps a slow runner and costs a fast one nothing.
const WAIT_TIMEOUT: Duration = Duration::from_secs(20);

/// A raw TLS connection to the server: stdin fed on demand, stdout
/// accumulated in the background so `wait_for`/`output` can poll it without
/// blocking.
struct RawConn {
    _child: ChildGuard,
    stdin: ChildStdin,
    output: Arc<Mutex<String>>,
}

impl RawConn {
    fn open(port: u16) -> Self {
        let mut child: Child = Command::new("openssl")
            .args([
                "s_client",
                "-quiet",
                "-verify_quiet",
                "-connect",
                &format!("127.0.0.1:{port}"),
                "-servername",
                "localhost",
            ])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .expect("spawning openssl s_client failed");
        let stdin = child.stdin.take().expect("child stdin");
        let mut stdout = child.stdout.take().expect("child stdout");
        let output = Arc::new(Mutex::new(String::new()));
        let output_writer = Arc::clone(&output);
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match stdout.read(&mut buf) {
                    Ok(0) => break,
                    Ok(n) => {
                        let text = String::from_utf8_lossy(&buf[..n]);
                        output_writer.lock().unwrap().push_str(&text);
                    }
                    Err(_) => break,
                }
            }
        });
        RawConn {
            _child: ChildGuard(child),
            stdin,
            output,
        }
    }

    fn feed(&mut self, burst: &str) {
        self.stdin
            .write_all(burst.as_bytes())
            .expect("writing to openssl s_client stdin failed");
        let _ = self.stdin.flush();
    }

    fn output(&self) -> String {
        self.output.lock().unwrap().clone()
    }

    fn wait_for(&self, needle: &str, timeout: Duration) -> bool {
        let deadline = Instant::now() + timeout;
        while Instant::now() < deadline {
            if self.output().contains(needle) {
                return true;
            }
            thread::sleep(Duration::from_millis(50));
        }
        false
    }
}

// ── CAP LS lists exactly the registered capabilities ────────────────────────
#[test]
fn cap_ls_lists_exactly_message_tags() {
    let server = spawn_server("message-tags-ls", &[]);
    let mut conn = RawConn::open(server.port);
    conn.feed("CAP LS\r\n");
    assert!(
        conn.wait_for("CAP * LS", WAIT_TIMEOUT),
        "CAP LS produced no reply at all: {}",
        conn.output()
    );
    assert!(
        conn.output().contains("CAP * LS :message-tags"),
        "CAP LS did not list exactly message-tags: {}",
        conn.output()
    );
}

// ── REQ of a supported capability ACKs ──────────────────────────────────────
#[test]
fn cap_req_of_a_supported_capability_acks() {
    let server = spawn_server("message-tags-req-ok", &[]);
    let mut conn = RawConn::open(server.port);
    conn.feed("CAP REQ :message-tags\r\n");
    assert!(
        conn.wait_for("CAP *", WAIT_TIMEOUT),
        "CAP REQ message-tags produced no reply"
    );
    assert!(
        conn.output().contains("CAP * ACK :message-tags"),
        "CAP REQ message-tags did not ACK: {}",
        conn.output()
    );
}

// ── REQ of an unsupported capability NAKs ───────────────────────────────────
#[test]
fn cap_req_of_an_unsupported_capability_naks() {
    let server = spawn_server("message-tags-req-bad", &[]);
    let mut conn = RawConn::open(server.port);
    conn.feed("CAP REQ :no-such-capability\r\n");
    assert!(
        conn.wait_for("CAP *", WAIT_TIMEOUT),
        "CAP REQ no-such-capability produced no reply"
    );
    assert!(
        conn.output().contains("CAP * NAK :no-such-capability"),
        "CAP REQ of an unsupported capability did not NAK: {}",
        conn.output()
    );
}

// ── a mixed REQ (one supported, one not) NAKs the whole request ────────────
#[test]
fn cap_req_mixed_naks_the_whole_set() {
    let server = spawn_server("message-tags-req-mixed", &[]);
    let mut conn = RawConn::open(server.port);
    conn.feed("CAP REQ :message-tags no-such-capability\r\n");
    assert!(
        conn.wait_for("CAP *", WAIT_TIMEOUT),
        "the mixed CAP REQ produced no reply"
    );
    assert!(
        conn.output()
            .contains("CAP * NAK :message-tags no-such-capability"),
        "a mixed CAP REQ did not NAK the whole set: {}",
        conn.output()
    );
}

// ── registration is held across CAP LS ... CAP END, and completes on END ───
#[test]
fn registration_is_held_across_cap_negotiation_until_end() {
    let server = spawn_server("message-tags-held", &[]);
    let mut conn = RawConn::open(server.port);
    conn.feed("CAP LS\r\nNICK capheld\r\nUSER capheld 0 * :capheld\r\n");
    thread::sleep(Duration::from_secs(2));
    assert!(
        !conn.output().contains(" 001 "),
        "registration completed before CAP END: {}",
        conn.output()
    );
    conn.feed("CAP END\r\n");
    assert!(
        conn.wait_for(" 001 ", WAIT_TIMEOUT),
        "registration never completed after CAP END: {}",
        conn.output()
    );
}

// ── a plain NICK/USER client with no CAP registers exactly as before ───────
#[test]
fn a_plain_client_with_no_cap_registers_normally() {
    let server = spawn_server("message-tags-plain", &[]);
    let mut conn = RawConn::open(server.port);
    conn.feed("NICK plainreg\r\nUSER plainreg 0 * :plainreg\r\n");
    assert!(
        conn.wait_for(" 001 ", WAIT_TIMEOUT),
        "a plain NICK/USER client (no CAP at all) never registered: {}",
        conn.output()
    );
}

// ── two connections negotiate independently ─────────────────────────────────
#[test]
fn two_connections_negotiate_independently() {
    let server = spawn_server("message-tags-indep", &[]);
    let mut a = RawConn::open(server.port);
    let mut b = RawConn::open(server.port);
    a.feed("CAP LS\r\nCAP REQ :message-tags\r\nNICK indepa\r\nUSER indepa 0 * :a\r\nCAP END\r\n");
    b.feed("NICK indepb\r\nUSER indepb 0 * :b\r\n");
    assert!(
        a.wait_for(" 001 ", WAIT_TIMEOUT),
        "connection A never registered: {}",
        a.output()
    );
    assert!(
        b.wait_for(" 001 ", WAIT_TIMEOUT),
        "connection B never registered: {}",
        b.output()
    );
    assert!(
        a.output().contains("CAP * ACK :message-tags"),
        "connection A's own negotiation did not ACK: {}",
        a.output()
    );
    assert!(
        !b.output().contains("CAP *"),
        "connection B, which never sent CAP, got a CAP reply anyway: {}",
        b.output()
    );
}

// ── T134: a negotiated peer gets the msgid tag, a plain peer in the SAME
//    channel does not, from the SAME broadcast ─────────────────────────────
#[test]
fn a_negotiated_peer_gets_msgid_a_plain_peer_does_not() {
    let server = spawn_server("message-tags-broadcast", &[]);
    let chan = "#capnego";
    let mut tagged = RawConn::open(server.port);
    let mut plain = RawConn::open(server.port);
    tagged.feed(&format!(
        "CAP LS\r\nCAP REQ :message-tags\r\nNICK tagpeer\r\nUSER tagpeer 0 * :t\r\nCAP END\r\nJOIN {chan}\r\n"
    ));
    plain.feed(&format!(
        "NICK plainpeer\r\nUSER plainpeer 0 * :p\r\nJOIN {chan}\r\n"
    ));
    assert!(
        tagged.wait_for("End of /NAMES list", WAIT_TIMEOUT),
        "the tagged peer never completed its JOIN: {}",
        tagged.output()
    );
    assert!(
        plain.wait_for("End of /NAMES list", WAIT_TIMEOUT),
        "the plain peer never completed its JOIN: {}",
        plain.output()
    );

    let mut sender = RawConn::open(server.port);
    sender.feed(&format!(
        "NICK capsender\r\nUSER capsender 0 * :s\r\nJOIN {chan}\r\nPRIVMSG {chan} :tagged-vs-plain\r\n"
    ));
    assert!(
        sender.wait_for("End of /NAMES list", WAIT_TIMEOUT),
        "the sender never joined: {}",
        sender.output()
    );

    assert!(
        tagged.wait_for("tagged-vs-plain", WAIT_TIMEOUT),
        "the tagged peer never received the broadcast at all: {}",
        tagged.output()
    );
    let tagged_out = tagged.output();
    assert!(
        tagged_out.contains("@msgid=")
            && tagged_out.contains(&format!("PRIVMSG {chan} :tagged-vs-plain")),
        "the message-tags-negotiated peer did not get a msgid tag: {tagged_out}"
    );

    assert!(
        plain.wait_for("tagged-vs-plain", WAIT_TIMEOUT),
        "the plain peer never received the broadcast at all: {}",
        plain.output()
    );
    let plain_out = plain.output();
    assert!(
        !plain_out.contains("@msgid="),
        "a peer that never negotiated message-tags got a tag anyway: {plain_out}"
    );
    assert!(
        plain_out.contains(":capsender!")
            && plain_out.contains(&format!("PRIVMSG {chan} :tagged-vs-plain")),
        "the plain peer's line did not match the expected untagged shape: {plain_out}"
    );
}

// ── B157/T135: a live tail's cursor advances from the real msgid tag, not
//    from a once-a-second poll ──────────────────────────────────────────────
#[test]
fn a_live_tail_cursor_advances_from_msgid_not_a_poll() {
    let server = spawn_server("tail-cursor", &[]);
    let chan = "#cursorcheck";
    let chan_log = server.home.join(&format!("channels/{chan}.log"));

    let tail_dir = support::ScratchDir::new("tail-cursor-tail");
    let binary = support::resolve_workspace_binary("chat-client-rs");
    let mut tail_child = Command::new(&binary)
        .args([
            "tail",
            "--server",
            &format!("127.0.0.1:{}", server.port),
            "--nick",
            "tailer",
            "--chan",
            chan,
            "--insecure",
        ])
        .env("AI_CHAT_HOME", tail_dir.path())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawning chat-client-rs tail failed");
    let tail_stderr = tail_child.stderr.take().unwrap();
    let tail_stderr_buf = Arc::new(Mutex::new(String::new()));
    {
        let buf = Arc::clone(&tail_stderr_buf);
        let mut stderr = tail_stderr;
        thread::spawn(move || {
            let mut text = String::new();
            let _ = stderr.read_to_string(&mut text);
            *buf.lock().unwrap() = text;
        });
    }
    let tail_guard = ChildGuard(tail_child);

    // Let registration, CAP negotiation and the JOIN complete before anything
    // is sent -- a broadcast is not replayed.
    thread::sleep(Duration::from_secs(1));

    let sender_dir = support::ScratchDir::new("tail-cursor-sender");
    for i in 1..=5 {
        let output = Command::new(&binary)
            .args([
                "send",
                "--server",
                &format!("127.0.0.1:{}", server.port),
                "--nick",
                "sender",
                "--chan",
                chan,
                "--text",
                &format!("rapid-{i}"),
                "--insecure",
                "--no-session",
            ])
            .env("AI_CHAT_HOME", sender_dir.path())
            .output()
            .expect("running chat-client-rs send failed");
        assert!(
            output.status.success(),
            "rapid send {i} failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let cursor_of = || -> Option<u64> {
        let output = Command::new(&binary)
            .args(["session", "cursor", chan])
            .env("AI_CHAT_HOME", tail_dir.path())
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&output.stdout);
        text.split_whitespace().nth(1)?.parse().ok()
    };
    let actual_max = || -> u64 {
        std::fs::read_to_string(&chan_log)
            .unwrap_or_default()
            .lines()
            .filter(|line| line.starts_with("MSG "))
            .count() as u64
    };

    // The cursor must reach the channel's last id well inside 1 second: the
    // old poll-only cadence resynchronizes at most once a second, so landing
    // here within 400ms is not explainable by polling.
    let deadline = Instant::now() + Duration::from_millis(400);
    let mut caught_up = false;
    while Instant::now() < deadline {
        if let Some(recorded) = cursor_of() {
            if recorded != 0 && recorded == actual_max() {
                caught_up = true;
                break;
            }
        }
        thread::sleep(Duration::from_millis(20));
    }
    assert!(
        caught_up,
        "the tail's cursor ({:?}) had not caught up to the channel's last id ({}) within 400ms of five rapid sends",
        cursor_of(),
        actual_max()
    );

    // Still alive and functioning -- nothing above bought this with a panic
    // or an early exit. `try_wait` returns `Ok(None)` while the child is
    // still running, without reaping it out from under the guard's own Drop.
    let mut tail_guard = tail_guard;
    assert!(
        matches!(tail_guard.0.try_wait(), Ok(None)),
        "the tail process was not still running"
    );
    let stderr_text = tail_stderr_buf.lock().unwrap().clone();
    assert!(
        !stderr_text.contains("panic"),
        "the tail logged a panic: {stderr_text}"
    );
}
