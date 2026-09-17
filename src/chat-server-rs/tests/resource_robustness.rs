// MODE: DEV
//! Server resource robustness under one wedged or many exhausted
//! connections, migrated from chat/tests/test-chat-broadcast-stall.sh (B122)
//! and chat/tests/test-chat-descriptor-leak.sh (B158/B159/B160, corrected
//! after AR-116 cycle 47 -- the bash file's own header cites B127, an
//! unrelated interactive-shell bug; verified against BUGS.json's own fix
//! text) (T145 goal 25, W134).

mod support;

use std::io::{Read, Write};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use support::{run_client, spawn_server, ChildGuard, ScratchDir};

fn server_state(pid: u32) -> String {
    Command::new("ps")
        .args(["-o", "state=", "-p", &pid.to_string()])
        .output()
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_default()
}

// ── B122: one idle subscriber must not wedge the server ────────────────────
#[test]
fn one_idle_subscriber_does_not_wedge_the_broadcast_path() {
    let server = spawn_server("stall", &[]);
    let binary = support::resolve_workspace_binary("chat-client-rs");

    // One subscriber, attached the way an agent attaches: `tail` JOINs the
    // channel and never reads history again -- a live connection whose own
    // thread holds its state, which is what the unfixed broadcast path
    // starved on.
    let sub_dir = ScratchDir::new("stall-sub");
    let mut sub_child = Command::new(&binary)
        .args([
            "tail",
            "--server",
            &format!("127.0.0.1:{}", server.port),
            "--nick",
            "stallsub",
            "--chan",
            "#stall",
            "--insecure",
            "--no-session",
        ])
        .env("AI_CHAT_HOME", sub_dir.path())
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawning the subscriber tail failed");
    let mut sub_stdout = sub_child.stdout.take().unwrap();
    let sub_log = Arc::new(Mutex::new(String::new()));
    {
        let log = Arc::clone(&sub_log);
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            while let Ok(n) = sub_stdout.read(&mut buf) {
                if n == 0 {
                    break;
                }
                log.lock()
                    .unwrap()
                    .push_str(&String::from_utf8_lossy(&buf[..n]));
            }
        });
    }
    let sub_pid = sub_child.id();
    let mut sub_guard = ChildGuard(sub_child);

    // Wait for the subscriber to actually join (NAMES lists it).
    let check_dir = ScratchDir::new("stall-check");
    let mut joined = false;
    for _ in 0..60 {
        let output = Command::new(&binary)
            .args([
                "names",
                "--server",
                &format!("127.0.0.1:{}", server.port),
                "--nick",
                "checker",
                "--chan",
                "#stall",
                "--insecure",
                "--no-session",
            ])
            .env("AI_CHAT_HOME", check_dir.path())
            .output()
            .unwrap();
        let members = String::from_utf8_lossy(&output.stdout);
        if members.lines().any(|l| l == "stallsub") {
            joined = true;
            break;
        }
        thread::sleep(Duration::from_millis(500));
    }
    assert!(joined, "the subscriber never joined #stall within 30s");

    // Precondition: the subscriber is connected and being serviced, proven
    // by it printing a message another client sent.
    let opener_dir = ScratchDir::new("stall-opener");
    let out = run_client(
        opener_dir.path(),
        &[
            "send",
            "--server",
            &format!("127.0.0.1:{}", server.port),
            "--nick",
            "opener",
            "--chan",
            "#stall",
            "--text",
            "subscriber-armed",
            "--insecure",
            "--no-session",
        ],
    );
    assert!(out.status.success());
    let mut attached = false;
    for _ in 0..60 {
        if sub_log.lock().unwrap().contains("subscriber-armed") {
            attached = true;
            break;
        }
        thread::sleep(Duration::from_millis(500));
    }
    assert!(
        attached,
        "the subscriber never received a message, so it was not a connection the server was servicing: [{}]",
        sub_log.lock().unwrap()
    );

    let logged = |chan: &str, text: &str| -> bool {
        std::fs::read_to_string(server.home.join(&format!("channels/{chan}.log")))
            .unwrap_or_default()
            .contains(text)
    };
    let send_as = |nick: &str, chan: &str, text: &str| -> std::process::Output {
        let dir = ScratchDir::new(&format!("stall-{nick}"));
        run_client(
            dir.path(),
            &[
                "send",
                "--server",
                &format!("127.0.0.1:{}", server.port),
                "--nick",
                nick,
                "--chan",
                chan,
                "--text",
                text,
                "--insecure",
                "--no-session",
            ],
        )
    };

    // 1. A fresh connection must still be serviced.
    let out = send_as("newcomer", "#other", "a-fresh-connection");
    assert!(
        out.status.success(),
        "a fresh connection was not serviced while one subscriber was attached: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        logged("#other", "a-fresh-connection"),
        "the fresh client's send returned but the message never reached the log"
    );

    // 2. Not merely slow: three consecutive sends must all land.
    for i in 1..=3 {
        let text = format!("consecutive-{i}");
        let out = send_as(&format!("sender{i}"), "#stall", &text);
        assert!(
            out.status.success(),
            "send {i} stalled while one subscriber was attached: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            logged("#stall", &text),
            "send {i} returned but message {text} never reached the log"
        );
    }

    // 3. A subscriber that is alive but not reading at all (SIGSTOP).
    let _ = Command::new("kill")
        .args(["-STOP", &sub_pid.to_string()])
        .status();
    let out = send_as("afterstop", "#stall", "after-the-subscriber-stopped");
    assert!(
        out.status.success(),
        "a stopped (alive, non-reading) subscriber stalled the server: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        logged("#stall", "after-the-subscriber-stopped"),
        "the send after SIGSTOP returned but never reached the log"
    );

    // 4. Recovery must not need a restart.
    let out = send_as("afterstop2", "#other", "still-serving-other-channels");
    assert!(
        out.status.success(),
        "the server stopped serving other channels while a subscriber was stopped: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        logged("#other", "still-serving-other-channels"),
        "the second send after SIGSTOP returned but never reached the log"
    );

    let _ = Command::new("kill")
        .args(["-CONT", &sub_pid.to_string()])
        .status();
    drop(sub_guard.0.kill());
    let _ = sub_guard.0.wait();

    // 5. Delivery still works, watched via a raw TLS peer (no shipped client
    //    consumes a broadcast: chat-client-rs reads history with FETCH and
    //    discards PRIVMSG lines).
    let mut watcher: Child = Command::new("openssl")
        .args([
            "s_client",
            "-quiet",
            "-verify_quiet",
            "-connect",
            &format!("127.0.0.1:{}", server.port),
            "-servername",
            "localhost",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawning the openssl watcher failed");
    let mut watcher_stdin: ChildStdin = watcher.stdin.take().unwrap();
    let mut watcher_stdout = watcher.stdout.take().unwrap();
    let watcher_log = Arc::new(Mutex::new(String::new()));
    {
        let log = Arc::clone(&watcher_log);
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            while let Ok(n) = watcher_stdout.read(&mut buf) {
                if n == 0 {
                    break;
                }
                log.lock()
                    .unwrap()
                    .push_str(&String::from_utf8_lossy(&buf[..n]));
            }
        });
    }
    watcher_stdin
        .write_all(b"NICK watcher\r\nUSER watcher 0 * :watcher\r\nJOIN #stall\r\n")
        .unwrap();
    let _watcher_guard = ChildGuard(watcher);

    let deadline = Instant::now() + Duration::from_secs(20);
    let mut watcher_joined = false;
    while Instant::now() < deadline {
        if watcher_log.lock().unwrap().contains("End of /NAMES list") {
            watcher_joined = true;
            break;
        }
        thread::sleep(Duration::from_millis(200));
    }
    assert!(
        watcher_joined,
        "the openssl watcher never completed a JOIN, so broadcast delivery was not tested: [{}]",
        watcher_log.lock().unwrap()
    );
    let out = send_as("broadcaster", "#stall", "delivered-by-broadcast");
    assert!(
        out.status.success(),
        "the send to the joined watcher stalled: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let deadline = Instant::now() + Duration::from_secs(20);
    let mut delivered = false;
    while Instant::now() < deadline {
        if watcher_log
            .lock()
            .unwrap()
            .contains("delivered-by-broadcast")
        {
            delivered = true;
            break;
        }
        thread::sleep(Duration::from_millis(200));
    }
    assert!(
        delivered,
        "a joined watcher never received the broadcast: [{}]",
        watcher_log.lock().unwrap()
    );

    // 6. Nothing above may have been bought with a panicked server thread.
    let state = server_state(server.child.pid());
    assert!(
        !state.is_empty() && !state.starts_with('Z'),
        "the server process died during the run (state '{state}')"
    );
}

// ── B158/B159/B160: no descriptor leak per connection; survives exhaustion ─
#[test]
fn no_descriptor_leak_and_survives_descriptor_exhaustion() {
    let binary_server = support::resolve_workspace_binary("chat-server-rs");
    let binary_client = support::resolve_workspace_binary("chat-client-rs");

    // A ulimit low enough that this host might refuse it is a SKIP for that
    // scenario, not a failure -- the point is the leak, not the exact number
    // a platform allows.
    let ulimit_probe = Command::new("sh").arg("-c").arg("ulimit -n 40").status();
    if !matches!(ulimit_probe, Ok(status) if status.success()) {
        eprintln!("SKIP: this host will not accept ulimit -n 40");
        return;
    }

    // 1 & 2. Under a low descriptor ceiling, every message must still be
    // accepted and appended, and the server must not hold a growing
    // descriptor table.
    let low_home = ScratchDir::new("fd-low");
    let mut low_server = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "ulimit -n 40 && exec env AI_CHAT_HOME={} CHAT_BEACON_PORT=47993 {} 0",
            shell_quote(low_home.path().to_str().unwrap()),
            shell_quote(binary_server.to_str().unwrap()),
        ))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawning the low-ceiling server failed");
    let low_pid = low_server.id();
    let low_port_file = low_home.join("server.port");
    let mut low_port = None;
    for _ in 0..40 {
        if let Ok(text) = std::fs::read_to_string(&low_port_file) {
            if let Ok(p) = text.trim().parse::<u16>() {
                low_port = Some(p);
                break;
            }
        }
        thread::sleep(Duration::from_millis(200));
    }
    if let Some(port) = low_port {
        let mut sent_ok = 0;
        for i in 1..=50 {
            let dir = ScratchDir::new(&format!("fd-{i}"));
            let out = run_client(
                dir.path(),
                &[
                    "send",
                    "--server",
                    &format!("127.0.0.1:{port}"),
                    "--nick",
                    &format!("fd{i}"),
                    "--chan",
                    "#fd",
                    "--text",
                    &format!("message-{i}"),
                    "--insecure",
                    "--no-session",
                ],
            );
            if out.status.success() {
                sent_ok += 1;
            }
        }
        let logged = std::fs::read_to_string(low_home.join("channels/#fd.log"))
            .unwrap_or_default()
            .lines()
            .count();
        assert_eq!(
            sent_ok, 50,
            "only {sent_ok} of 50 sends completed under a 40-descriptor limit"
        );
        assert_eq!(
            logged, 50,
            "only {logged} of 50 messages were appended under a 40-descriptor limit - the rest were accepted-and-lost"
        );

        if std::path::Path::new(&format!("/proc/{low_pid}/fd")).is_dir() {
            let after = std::fs::read_dir(format!("/proc/{low_pid}/fd"))
                .map(|entries| entries.count())
                .unwrap_or(0);
            assert!(
                after <= 12,
                "the server holds {after} descriptors after 50 connections (expected a handful): one is leaking per connection"
            );
        } else {
            eprintln!("SKIP: no /proc, so the descriptor count itself was not asserted");
        }
    } else {
        panic!(
            "the server did not start under a 40-descriptor limit: {}",
            wait_and_read_stderr(&mut low_server)
        );
    }
    let _ = low_server.kill();
    let _ = low_server.wait();

    // 3. Running out of descriptors must be survivable and reported, not a
    //    panic (this crate is built with panic = "abort", so an abort here
    //    would take every other live connection with it).
    let tiny_home = ScratchDir::new("fd-tiny");
    let mut tiny_server = Command::new("sh")
        .arg("-c")
        .arg(format!(
            "ulimit -n 6 && exec env AI_CHAT_HOME={} CHAT_BEACON_PORT=47993 {} 0",
            shell_quote(tiny_home.path().to_str().unwrap()),
            shell_quote(binary_server.to_str().unwrap()),
        ))
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawning the tiny-ceiling server failed");
    let tiny_pid = tiny_server.id();
    let tiny_port_file = tiny_home.join("server.port");
    let mut tiny_port = None;
    for _ in 0..40 {
        if let Ok(text) = std::fs::read_to_string(&tiny_port_file) {
            if let Ok(p) = text.trim().parse::<u16>() {
                tiny_port = Some(p);
                break;
            }
        }
        thread::sleep(Duration::from_millis(200));
    }
    if let Some(port) = tiny_port {
        let dir = ScratchDir::new("fd-tiny-send");
        let _ = Command::new(&binary_client)
            .args([
                "send",
                "--server",
                &format!("127.0.0.1:{port}"),
                "--nick",
                "tiny",
                "--chan",
                "#fd",
                "--text",
                "this one cannot be served",
                "--insecure",
                "--no-session",
            ])
            .env("AI_CHAT_HOME", dir.path())
            .output();
        thread::sleep(Duration::from_secs(1));
        let state = server_state(tiny_pid);
        assert!(
            !state.is_empty() && !state.starts_with('Z'),
            "the server died when it ran out of descriptors instead of refusing the connection (state '{}')",
            if state.is_empty() { "gone" } else { &state }
        );
        let stderr_text = wait_and_read_stderr_nonblocking(&mut tiny_server);
        if std::path::Path::new("/proc").is_dir() {
            assert!(
                stderr_text.contains("cannot accept connections"),
                "the server never reported that it could not accept connections; stderr was: {stderr_text}"
            );
        } else {
            eprintln!("SKIP: no /proc, so exhaustion could not be confirmed");
        }
        assert!(
            !stderr_text.contains("panicked"),
            "the server panicked on descriptor exhaustion instead of reporting it: {stderr_text}"
        );
    } else {
        eprintln!("SKIP: the server would not start at ulimit -n 6 on this host");
    }
    let _ = tiny_server.kill();
    let _ = tiny_server.wait();
}

fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

fn wait_and_read_stderr(child: &mut Child) -> String {
    let mut text = String::new();
    if let Some(stderr) = child.stderr.as_mut() {
        let _ = stderr.read_to_string(&mut text);
    }
    text
}

fn wait_and_read_stderr_nonblocking(child: &mut Child) -> String {
    // The process is still running (or has just died); reading its stderr
    // pipe here would block until it closes. Give it a moment to write, then
    // read whatever is already buffered via a short-lived thread with a
    // timeout join.
    thread::sleep(Duration::from_millis(500));
    if let Some(mut stderr) = child.stderr.take() {
        let (tx, rx) = std::sync::mpsc::channel();
        thread::spawn(move || {
            let mut buf = [0u8; 8192];
            let mut text = String::new();
            // Non-blocking-ish: one read attempt is enough since the server
            // writes its refusal synchronously before this point.
            if let Ok(n) = stderr.read(&mut buf) {
                text.push_str(&String::from_utf8_lossy(&buf[..n]));
            }
            let _ = tx.send(text);
        });
        rx.recv_timeout(Duration::from_secs(2)).unwrap_or_default()
    } else {
        String::new()
    }
}
