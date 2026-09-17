// MODE: DEV
//! T107/B283: one running `tail` binds a control socket in the session state
//! dir and serves the other verbs on ITS connection, so a send no longer
//! registers a second time under the same nick and the message arrives from
//! the nick the agent chose. Migrated from chat/tests/test-chat-owner-socket.sh
//! (T145 goal 25, W135) -- chat-client-rs's first tests/ directory.
//!
//! Reuses W131's shared support module from chat-server-rs's own tests/
//! directory via `#[path]` rather than a new Cargo dependency: neither crate
//! depends on the other in `[dependencies]`, and a real (even dev-only) path
//! dependency between them would be a heavier answer than sharing one test
//! module needs (05-step-migrate-owner-socket's own handoff).

#[path = "../../chat-server-rs/tests/support/mod.rs"]
mod support;

use std::os::unix::fs::PermissionsExt;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use support::{spawn_server, ChildGuard};

fn permission_bits(path: &std::path::Path) -> u32 {
    std::fs::metadata(path).unwrap().permissions().mode() & 0o777
}

#[test]
fn a_running_tail_owns_its_connection_and_serves_verbs_on_it() {
    let server = spawn_server("owner-socket", &[("CHAT_ANNOUNCE", "0")]);
    let binary = support::resolve_workspace_binary("chat-client-rs");
    // ONE shared AI_CHAT_HOME for the server and every client/tail below --
    // not a separate scratch dir for the client side. AI_CHAT_HOME serves
    // double duty (server-side channel storage AND client-side
    // session/owner-record state), and the bash original relies on exactly
    // that: a tail's own control-socket record and the channel log it
    // forwards into must live under the SAME home the server was started
    // with, or the two halves of this test never see each other's state.
    let home = server.home.path();
    let chan = "#owned";
    let nick = "owner";
    let key = "t107";
    let record = home.join(format!("owners/{key}.json"));

    let owner_socket = || -> Option<String> {
        let text = std::fs::read_to_string(&record).ok()?;
        let value: serde_json::Value = serde_json::from_str(&text).ok()?;
        value
            .get("socket")
            .and_then(|s| s.as_str())
            .map(|s| s.to_string())
    };
    let await_socket = || -> Option<String> {
        for _ in 0..50 {
            if let Some(path) = owner_socket() {
                if std::path::Path::new(&path).exists() {
                    return Some(path);
                }
            }
            thread::sleep(Duration::from_millis(200));
        }
        owner_socket()
    };

    let client = |args: &[&str]| -> std::process::Output {
        let mut full = vec!["--session", key];
        full.extend_from_slice(args);
        full.push("--insecure");
        Command::new(&binary)
            .args(full)
            .env("AI_CHAT_HOME", home)
            .output()
            .expect("running chat-client-rs failed")
    };
    let peer_send = |chan: &str, text: &str| {
        Command::new(&binary)
            .args([
                "send",
                "--server",
                &format!("127.0.0.1:{}", server.port),
                "--nick",
                "peer",
                "--chan",
                chan,
                "--text",
                text,
                "--no-session",
                "--insecure",
            ])
            .env("AI_CHAT_HOME", home)
            .output()
            .unwrap();
    };

    client(&[
        "session",
        "set",
        "--server",
        &format!("127.0.0.1:{}", server.port),
        "--nick",
        nick,
    ]);
    client(&["join", "--chan", chan]);

    // ── the tail takes ownership ─────────────────────────────────────────
    let mut tail_child = Command::new(&binary)
        .args([
            "--session",
            key,
            "tail",
            "--chan",
            chan,
            "--presence",
            "--insecure",
        ])
        .env("AI_CHAT_HOME", home)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawning the owner tail failed");
    let mut tail_stdout = tail_child.stdout.take().unwrap();
    let tail_log = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    {
        let log = std::sync::Arc::clone(&tail_log);
        thread::spawn(move || {
            use std::io::Read;
            let mut buf = [0u8; 4096];
            while let Ok(n) = tail_stdout.read(&mut buf) {
                if n == 0 {
                    break;
                }
                log.lock()
                    .unwrap()
                    .push_str(&String::from_utf8_lossy(&buf[..n]));
            }
        });
    }
    let tail_pid = tail_child.id();
    let mut tail_guard = ChildGuard(tail_child);

    let socket = await_socket();
    assert!(
        socket
            .as_deref()
            .is_some_and(|s| std::path::Path::new(s).exists()),
        "the tail bound no control socket (record: {:?})",
        std::fs::read_to_string(&record).ok()
    );
    let socket = socket.unwrap();
    let socket_path = std::path::Path::new(&socket);

    assert_eq!(
        permission_bits(&home.join("owners")),
        0o700,
        "the record directory is private"
    );
    assert_eq!(
        permission_bits(socket_path.parent().unwrap()),
        0o700,
        "the socket directory is private"
    );
    assert_eq!(
        permission_bits(socket_path),
        0o600,
        "the socket is owner-only"
    );
    assert!(
        socket.len() < 100,
        "the socket path is not well inside the address limit: {} chars",
        socket.len()
    );

    // ── a forwarded send is not suffixed (B283) ─────────────────────────
    let out = client(&["send", "--chan", chan, "--text", "through the owner"]);
    assert!(
        out.status.success(),
        "a forwarded send failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let channel_log = || -> String {
        std::fs::read_to_string(home.join("channels").join(format!("{chan}.log")))
            .unwrap_or_default()
    };
    let sender_of = |fragment: &str| -> Option<String> {
        channel_log()
            .lines()
            .rfind(|line| line.contains(fragment))
            .and_then(|line| line.split_whitespace().nth(4))
            .map(|s| s.to_string())
    };
    // The client's own exit only means the send handed off to the owner's
    // control socket; the tail's own connection then relays it to the server,
    // which appends to the log slightly after that -- poll rather than assert
    // immediately.
    let wait_for_sender = |fragment: &str, timeout: Duration| -> Option<String> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(found) = sender_of(fragment) {
                return Some(found);
            }
            if Instant::now() >= deadline {
                return None;
            }
            thread::sleep(Duration::from_millis(100));
        }
    };
    assert_eq!(
        wait_for_sender("through the owner", Duration::from_secs(5)).as_deref(),
        Some(nick),
        "a send through the owner arrives from the unsuffixed nick"
    );

    // The positive control: --no-session bypasses the socket, so this send
    // registers a second time under a nick the tail is already holding, and
    // the server suffixes it.
    let out = Command::new(&binary)
        .args([
            "send",
            "--server",
            &format!("127.0.0.1:{}", server.port),
            "--nick",
            nick,
            "--chan",
            chan,
            "--text",
            "around the owner",
            "--no-session",
            "--insecure",
        ])
        .env("AI_CHAT_HOME", home)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "the bypassing send failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(
        wait_for_sender("around the owner", Duration::from_secs(5)).as_deref(),
        Some(format!("{nick}-2").as_str()),
        "a send that bypasses the owner is still suffixed by the server"
    );

    // ── names is served on the same connection ──────────────────────────
    let names_out =
        String::from_utf8_lossy(&client(&["names", "--chan", chan]).stdout).into_owned();
    let count = names_out.lines().filter(|l| l.trim() == nick).count();
    assert_eq!(
        count, 1,
        "names served by the owner should list the tailing nick exactly once, got {count}"
    );

    // ── read is served, and reports what the wire holds ─────────────────
    let read_out =
        String::from_utf8_lossy(&client(&["read", "--chan", chan, "--since", "0"]).stdout)
            .into_owned();
    let matched = read_out
        .lines()
        .filter(|l| l.contains("through the owner") || l.contains("around the owner"))
        .count();
    assert_eq!(
        matched, 2,
        "read served by the owner should return the persisted messages, got {matched}"
    );

    // ── a forwarded join is FOLLOWED, not just joined (B296) ────────────
    let second = "#alsoowned";
    let join2 = client(&["join", "--chan", second]);
    assert!(
        join2.status.success(),
        "a forwarded join failed: {}",
        String::from_utf8_lossy(&join2.stderr)
    );
    let join2_out = String::from_utf8_lossy(&join2.stdout).into_owned();
    assert!(
        join2_out.contains("following")
            && join2_out.contains(&chan[1..])
            && join2_out.contains(&second[1..]),
        "a forwarded join reports the whole followed set: [{join2_out}]"
    );

    peer_send(second, "into the joined channel");
    let deadline = Instant::now() + Duration::from_secs(6);
    let mut followed = false;
    while Instant::now() < deadline {
        if tail_log.lock().unwrap().contains("into the joined channel") {
            followed = true;
            break;
        }
        thread::sleep(Duration::from_millis(200));
    }
    assert!(
        followed,
        "the tail should print traffic from a channel joined after it started"
    );

    // ── leave drops one channel and keeps the tail on the rest ──────────
    let leave2 = client(&["leave", "--chan", second]);
    assert!(
        leave2.status.success(),
        "leave on a followed channel failed: {}",
        String::from_utf8_lossy(&leave2.stderr)
    );
    assert!(
        String::from_utf8_lossy(&leave2.stdout).contains("still following"),
        "leave should say what is still followed"
    );
    assert!(
        matches!(tail_guard.0.try_wait(), Ok(None)),
        "leaving one of two channels stopped the tail"
    );
    assert!(
        socket_path.exists(),
        "leaving one of two channels took the control socket down"
    );

    // ── leaving the LAST channel stops the tail ─────────────────────────
    let leave1 = client(&["leave", "--chan", chan]);
    assert!(
        leave1.status.success(),
        "leave on the last channel failed: {}",
        String::from_utf8_lossy(&leave1.stderr)
    );
    assert!(
        String::from_utf8_lossy(&leave1.stdout).contains("the tail is stopping"),
        "leave on the last channel should say the tail is stopping"
    );
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut stopped = false;
    while Instant::now() < deadline {
        if matches!(tail_guard.0.try_wait(), Ok(Some(_))) {
            stopped = true;
            break;
        }
        thread::sleep(Duration::from_millis(250));
    }
    assert!(stopped, "the tail did not actually exit");
    assert!(
        !socket_path.exists(),
        "the control socket should be gone once the tail exits"
    );
    let _ = tail_pid;

    // ── with no owner at all, every verb works as before T107 ───────────
    let after = client(&["send", "--chan", chan, "--text", "after the owner"]);
    assert!(
        after.status.success(),
        "a send with no owner failed: {}",
        String::from_utf8_lossy(&after.stderr)
    );
    assert_eq!(
        wait_for_sender("after the owner", Duration::from_secs(5)).as_deref(),
        Some(nick),
        "a send with no owner running should still reach the channel"
    );

    // ── a later tail takes the socket the stopped one released ─────────
    let mut tail2_child = Command::new(&binary)
        .args([
            "--session",
            key,
            "tail",
            "--chan",
            chan,
            "--chan",
            second,
            "--insecure",
        ])
        .env("AI_CHAT_HOME", home)
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawning the second tail failed");
    let mut tail2_stdout = tail2_child.stdout.take().unwrap();
    let tail2_log = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    {
        let log = std::sync::Arc::clone(&tail2_log);
        thread::spawn(move || {
            use std::io::Read;
            let mut buf = [0u8; 4096];
            while let Ok(n) = tail2_stdout.read(&mut buf) {
                if n == 0 {
                    break;
                }
                log.lock()
                    .unwrap()
                    .push_str(&String::from_utf8_lossy(&buf[..n]));
            }
        });
    }
    let tail2_guard = ChildGuard(tail2_child);
    let mut took_socket = false;
    for _ in 0..50 {
        if socket_path.exists() {
            took_socket = true;
            break;
        }
        thread::sleep(Duration::from_millis(200));
    }
    assert!(
        took_socket,
        "a later tail should take the socket the stopped one released"
    );

    peer_send(chan, "first of two");
    peer_send(second, "second of two");
    let deadline = Instant::now() + Duration::from_secs(8);
    let mut both = false;
    while Instant::now() < deadline {
        let log = tail2_log.lock().unwrap();
        if log.contains("first of two") && log.contains("second of two") {
            both = true;
            break;
        }
        drop(log);
        thread::sleep(Duration::from_millis(200));
    }
    assert!(
        both,
        "a tail started with two --chan should print traffic from both"
    );

    // ── a CRASHED owner leaves its socket, and callers still fall back ──
    let mut tail2_guard = tail2_guard;
    let _ = tail2_guard.0.kill();
    let _ = tail2_guard.0.wait();
    let after_crash = client(&["send", "--chan", chan, "--text", "after the crash"]);
    assert!(
        after_crash.status.success(),
        "a send after the owner was killed failed: {}",
        String::from_utf8_lossy(&after_crash.stderr)
    );
    assert_eq!(
        wait_for_sender("after the crash", Duration::from_secs(5)).as_deref(),
        Some(nick),
        "a send after a crashed owner should still reach the channel"
    );
}
