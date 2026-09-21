// MODE: DEV
//! chat-server-rs/chat-client-rs end-to-end flow, migrated from
//! chat/tests/test-chat.sh (T145 goal 25, W132). Split into one #[test] per
//! phase (rather than one monolithic test) so a single failure identifies
//! which phase broke -- discovery, TLS-only enforcement, send/TOFU,
//! read-delta, fail-closed TOFU, idle-tail wake, sessions/cursors,
//! join/leave, mentions, per-agent sessions sharing one home, dead-peer
//! teardown, serverless local reads, the local channel-name guard, local
//! cursors, and --version/malformed-argv refusal. Every phase spawns the
//! real compiled binaries via `support::spawn_server`/`support::run_client`
//! and asserts on real process exit status, stdout/stderr, and the real
//! on-disk channel log -- no mocking any layer the bash original exercised
//! for real.

mod support;

use std::io::{Read, Write};
use std::net::TcpStream;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use support::{
    free_udp_port, run_client, spawn_server, stderr_string, stdout_string, ChatServer, ChildGuard,
    ScratchDir,
};

fn client_dir(server_home: &ScratchDir, suffix: &str) -> std::path::PathBuf {
    let dir = server_home.path().join(format!("c_{suffix}"));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

// ---- discovery -------------------------------------------------------------
#[test]
fn discovery_finds_the_announcing_server() {
    let beacon_port = free_udp_port();
    let server = spawn_server(
        "e2e-disco",
        &[
            ("CHAT_ANNOUNCE", "1"),
            ("CHAT_BCAST", "127.0.0.1"),
            ("CHAT_BEACON_PORT", beacon_port.as_str()),
            ("CHAT_NAME", "test-beacon"),
        ],
    );
    let dir = client_dir(&server.home, "disco");
    let output = run_client(
        &dir,
        &[
            "discover",
            "--bcast",
            "127.0.0.1",
            "--beacon-port",
            &beacon_port,
            "--wait",
            "3",
            "--json",
        ],
    );
    let text = stdout_string(&output);
    let port_field = format!(r#""port":{}"#, server.port);
    assert!(
        text.contains(&port_field),
        "discovery did not list the announcing server: {text}"
    );
}

// ---- the server must be TLS-only -------------------------------------------
#[test]
fn the_server_refuses_a_plain_non_tls_registration() {
    let server = spawn_server("e2e-tls-only", &[]);
    let mut stream = TcpStream::connect(("127.0.0.1", server.port))
        .expect("plain TCP connect to the TLS listener failed");
    stream
        .write_all(b"NICK plainprobe\r\nUSER plainprobe 0 * :plainprobe\r\n")
        .expect("writing the plain probe failed");
    stream
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    let mut buf = [0u8; 256];
    let mut reply = Vec::new();
    // A TLS listener reads a plaintext IRC line as a malformed record and
    // closes; a read timing out with nothing at all is exactly that shape, and
    // is not itself a failure -- only a real welcome numeric is.
    if let Ok(n) = stream.read(&mut buf) {
        reply.extend_from_slice(&buf[..n]);
    }
    let reply = String::from_utf8_lossy(&reply);
    assert!(
        !reply.contains(" 001 ") && !reply.contains("Welcome"),
        "the server registered a plain, non-TLS client: [{reply}]"
    );
}

// ---- send pins the cert (TOFU) and echoes the message ----------------------
#[test]
fn send_pins_the_cert_and_echoes_the_message() {
    let server = spawn_server("e2e-send", &[]);
    let dir = client_dir(&server.home, "sA");
    let output = run_client(
        &dir,
        &[
            "send",
            "--server",
            &format!("127.0.0.1:{}", server.port),
            "--nick",
            "alice",
            "--chan",
            "#ops",
            "--text",
            "hello rust chat",
        ],
    );
    let sent = stdout_string(&output);
    assert!(
        sent.contains(":alice!alice@localhost PRIVMSG #ops :hello rust chat"),
        "send did not echo the message: [{sent}]"
    );
    let fingerprint_pinned = std::fs::read_dir(&dir)
        .unwrap()
        .filter_map(|entry| entry.ok())
        .any(|entry| entry.file_name().to_string_lossy().ends_with(".cert.fp"));
    assert!(fingerprint_pinned, "no TOFU fingerprint file was pinned");
}

// ---- read-delta returns the message and the mismatched pin fails closed ---
#[test]
fn read_delta_returns_the_message_and_a_mismatched_pin_fails_closed() {
    let server = spawn_server("e2e-delta", &[]);
    let send_dir = client_dir(&server.home, "sA");
    run_client(
        &send_dir,
        &[
            "send",
            "--server",
            &format!("127.0.0.1:{}", server.port),
            "--nick",
            "alice",
            "--chan",
            "#ops",
            "--text",
            "hello rust chat",
        ],
    );
    let read_dir = client_dir(&server.home, "rB");
    let output = run_client(
        &read_dir,
        &[
            "read",
            "--server",
            &format!("127.0.0.1:{}", server.port),
            "--nick",
            "alice",
            "--chan",
            "#ops",
            "--since",
            "0",
        ],
    );
    let delta = stdout_string(&output);
    assert!(
        delta.contains("MSG #ops 1 ") && delta.contains(":hello rust chat"),
        "read-delta did not return the message: [{delta}]"
    );

    // The mismatched-pin fail-closed path (TOFU).
    let fingerprint = read_dir.join(format!("127_0_0_1_{}.cert.fp", server.port));
    std::fs::write(&fingerprint, "bogus\n").unwrap();
    let output = run_client(
        &read_dir,
        &[
            "read",
            "--server",
            &format!("127.0.0.1:{}", server.port),
            "--nick",
            "alice",
            "--chan",
            "#ops",
            "--since",
            "0",
        ],
    );
    assert_eq!(
        output.status.code(),
        Some(70),
        "mismatched TOFU pin did not fail closed: {}",
        stderr_string(&output)
    );
}

// ---- an idle tail stays alive and wakes on a pushed message ----------------
#[test]
fn an_idle_tail_survives_silence_and_wakes_on_a_pushed_message() {
    let server = spawn_server("e2e-wake", &[]);
    let wake_home = client_dir(&server.home, "wakehome");
    let binary = support::resolve_workspace_binary("chat-client-rs");
    let mut wake_child = Command::new(&binary)
        .args([
            "tail",
            "--server",
            &format!("127.0.0.1:{}", server.port),
            "--nick",
            "wakee",
            "--chan",
            "#wake",
            "--insecure",
        ])
        .env("AI_CHAT_HOME", &wake_home)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawning the idle tail failed");
    let mut wake_stdout = wake_child.stdout.take().unwrap();
    let log = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    {
        let log = std::sync::Arc::clone(&log);
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            loop {
                match wake_stdout.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => log
                        .lock()
                        .unwrap()
                        .push_str(&String::from_utf8_lossy(&buf[..n])),
                }
            }
        });
    }
    let mut wake_guard = ChildGuard(wake_child);
    thread::sleep(Duration::from_secs(2));
    assert!(
        matches!(wake_guard.0.try_wait(), Ok(None)),
        "idle tail did not start"
    );
    // Survive a silence past the old 5s read timeout.
    thread::sleep(Duration::from_secs(6));
    assert!(
        matches!(wake_guard.0.try_wait(), Ok(None)),
        "idle tail exited during a 6s silence"
    );
    let sender_dir = client_dir(&server.home, "wS");
    let send_output = run_client(
        &sender_dir,
        &[
            "send",
            "--server",
            &format!("127.0.0.1:{}", server.port),
            "--nick",
            "waker",
            "--chan",
            "#wake",
            "--text",
            "wake wakee now",
        ],
    );
    assert!(
        send_output.status.success(),
        "wake send failed: {}",
        stderr_string(&send_output)
    );
    let deadline = Instant::now() + Duration::from_secs(3);
    let mut woke = false;
    while Instant::now() < deadline {
        if log.lock().unwrap().contains("wake wakee now") {
            woke = true;
            break;
        }
        thread::sleep(Duration::from_millis(100));
    }
    assert!(
        woke,
        "idle tail did not wake on the message: [{}]",
        log.lock().unwrap()
    );
    assert!(
        matches!(wake_guard.0.try_wait(), Ok(None)),
        "tail exited after waking"
    );

    // Continuous pushes: every message surfaced exactly once.
    for i in 1..=8 {
        let push_dir = client_dir(&server.home, &format!("push{i}"));
        let out = run_client(
            &push_dir,
            &[
                "send",
                "--server",
                &format!("127.0.0.1:{}", server.port),
                "--nick",
                "pusher",
                "--chan",
                "#wake",
                "--text",
                &format!("continuous push {i}"),
            ],
        );
        assert!(
            out.status.success(),
            "continuous push {i} failed: {}",
            stderr_string(&out)
        );
    }
    thread::sleep(Duration::from_secs(3));
    let final_log = log.lock().unwrap().clone();
    for i in 1..=8 {
        let expected = format!(":pusher!pusher@localhost PRIVMSG #wake :continuous push {i}");
        let count = final_log.matches(&expected).count();
        assert_eq!(
            count, 1,
            "continuous push {i} surfaced {count} times instead of once"
        );
    }
}

// ---- sessions: set/show, send omits server/nick, send does not move the
//      read cursor (B254), join seeds the cursor to the end, leave drops it --
#[test]
fn sessions_persist_server_and_nick_and_send_never_moves_the_cursor() {
    let server = spawn_server("e2e-session", &[]);
    let binary = support::resolve_workspace_binary("chat-client-rs");
    let session_home = client_dir(&server.home, "sess");

    let cli = |args: &[&str]| -> std::process::Output {
        Command::new(&binary)
            .args(args)
            .env("AI_CHAT_HOME", &session_home)
            .output()
            .expect("running chat-client-rs failed")
    };

    cli(&[
        "session",
        "set",
        "--server",
        &format!("127.0.0.1:{}", server.port),
        "--nick",
        "sessioner",
    ]);
    let shown = stdout_string(&cli(&["session", "show"]));
    assert!(
        shown.contains(&format!("server=127.0.0.1:{}", server.port))
            && shown.contains("nick=sessioner"),
        "session set/show did not persist server+nick: [{shown}]"
    );

    // send WITHOUT --server/--nick uses the session.
    let sent2 = stdout_string(&cli(&[
        "send",
        "--chan",
        "#sess",
        "--text",
        "session message",
        "--insecure",
    ]));
    assert!(
        sent2.contains(":sessioner!sessioner@localhost PRIVMSG #sess :session message"),
        "session-backed send failed: [{sent2}]"
    );

    // B254: a send does not move the read cursor.
    cli(&[
        "send",
        "--chan",
        "#sess",
        "--text",
        "second session",
        "--insecure",
    ]);
    let shown = stdout_string(&cli(&["session", "show"]));
    let cursor = shown
        .lines()
        .find(|line| line.starts_with("cursor #sess"))
        .and_then(|line| line.split_whitespace().last());
    assert!(
        matches!(cursor, None | Some("0")),
        "send moved the read cursor to {cursor:?}; sending is not reading (B254)"
    );

    // A message that arrived before the send is still unread afterwards.
    cli(&["join", "--chan", "#sess", "--insecure"]);
    let other_dir = client_dir(&server.home, "other");
    let out = Command::new(&binary)
        .args([
            "send",
            "--chan",
            "#sess",
            "--server",
            &format!("127.0.0.1:{}", server.port),
            "--nick",
            "othersender",
            "--text",
            "arrived before the send",
            "--insecure",
        ])
        .env("AI_CHAT_HOME", &other_dir)
        .output()
        .unwrap();
    assert!(out.status.success());
    cli(&[
        "send",
        "--chan",
        "#sess",
        "--text",
        "third session",
        "--insecure",
    ]);
    let still_unread = stdout_string(&cli(&["read", "--chan", "#sess", "--insecure"]));
    assert!(
        still_unread.contains("arrived before the send"),
        "a message that arrived before the send was consumed by it: [{still_unread}]"
    );

    // A malformed session file must recover (warning + empty session).
    let sess_file = stdout_string(&cli(&["session", "show"]))
        .lines()
        .find_map(|line| line.strip_prefix("file="))
        .map(|s| s.to_string());
    let sess_file = sess_file.expect("session show did not report its file");
    std::fs::write(&sess_file, "{ broken json !!!").unwrap();
    let recovered = stdout_string(&cli(&["session", "show"]));
    assert!(
        recovered.contains("server="),
        "malformed session did not recover: [{recovered}]"
    );

    // --no-session ignores the saved server/nick (send without them fails).
    let out = Command::new(&binary)
        .args([
            "send",
            "--chan",
            "#x",
            "--text",
            "x",
            "--insecure",
            "--no-session",
        ])
        .env("AI_CHAT_HOME", &session_home)
        .output()
        .unwrap();
    assert_eq!(
        out.status.code(),
        Some(64),
        "--no-session send without server/nick exited {:?} (want 64)",
        out.status.code()
    );

    // join seeds the cursor to the channel's CURRENT end; read after join
    // resumes from the cursor; explicit --since 0 still reads everything;
    // leave drops the cursor.
    cli(&[
        "session",
        "set",
        "--server",
        &format!("127.0.0.1:{}", server.port),
        "--nick",
        "sessioner",
    ]);
    cli(&["send", "--chan", "#old", "--text", "ancient", "--insecure"]);
    cli(&["send", "--chan", "#old", "--text", "elder", "--insecure"]);
    let joined = stdout_string(&cli(&["join", "--chan", "#old", "--insecure"]));
    assert!(
        joined.contains("resuming after id 2"),
        "join did not seed to current end: [{joined}]"
    );
    let after = stdout_string(&cli(&["read", "--chan", "#old", "--insecure"]));
    assert!(
        after.is_empty(),
        "read after join dumped history: [{after}]"
    );
    let hist = stdout_string(&cli(&[
        "read",
        "--chan",
        "#old",
        "--since",
        "0",
        "--insecure",
    ]));
    assert!(
        hist.contains("ancient") && hist.contains("elder"),
        "read --since 0 did not return full history: [{hist}]"
    );
    cli(&["leave", "--chan", "#old", "--insecure"]);
    let shown = stdout_string(&cli(&["session", "show"]));
    assert!(
        !shown.lines().any(|line| line.starts_with("cursor #old")),
        "leave did not drop the #old cursor: [{shown}]"
    );
}

// ---- B269: a tail resumed with a recorded 0 cursor backfills -------------
#[test]
fn a_tail_resumed_with_a_recorded_zero_cursor_backfills() {
    let server = spawn_server("e2e-b269", &[]);
    let binary = support::resolve_workspace_binary("chat-client-rs");
    let dir = client_dir(&server.home, "b269");
    let joined = stdout_string(
        &Command::new(&binary)
            .args([
                "join",
                "--server",
                &format!("127.0.0.1:{}", server.port),
                "--nick",
                "backfiller",
                "--chan",
                "#backfill",
                "--insecure",
            ])
            .env("AI_CHAT_HOME", &dir)
            .output()
            .unwrap(),
    );
    assert!(
        joined.contains("resuming after id 0"),
        "B269 setup: join did not seed cursor 0 on an empty channel: [{joined}]"
    );
    let poster_dir = client_dir(&server.home, "poster269");
    run_client(
        &poster_dir,
        &[
            "send",
            "--server",
            &format!("127.0.0.1:{}", server.port),
            "--nick",
            "poster269",
            "--chan",
            "#backfill",
            "--text",
            "missed-while-away",
            "--insecure",
        ],
    );
    let mut tail_child = Command::new(&binary)
        .args([
            "tail",
            "--server",
            &format!("127.0.0.1:{}", server.port),
            "--nick",
            "backfiller",
            "--chan",
            "#backfill",
            "--insecure",
        ])
        .env("AI_CHAT_HOME", &dir)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdout = tail_child.stdout.take().unwrap();
    let log = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    {
        let log = std::sync::Arc::clone(&log);
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            while let Ok(n) = stdout.read(&mut buf) {
                if n == 0 {
                    break;
                }
                log.lock()
                    .unwrap()
                    .push_str(&String::from_utf8_lossy(&buf[..n]));
            }
        });
    }
    let _guard = ChildGuard(tail_child);
    thread::sleep(Duration::from_secs(2));
    assert!(
        log.lock().unwrap().contains("missed-while-away"),
        "B269: a tail resumed from a recorded 0 cursor did not backfill: [{}]",
        log.lock().unwrap()
    );
}

// ---- B295: a mentions-filtered remote read must not move the cursor ------
#[test]
fn a_mentions_filtered_read_does_not_move_the_cursor() {
    let server = spawn_server("e2e-b295", &[]);
    let binary = support::resolve_workspace_binary("chat-client-rs");
    let dir = client_dir(&server.home, "b295");
    Command::new(&binary)
        .args([
            "join",
            "--server",
            &format!("127.0.0.1:{}", server.port),
            "--nick",
            "b295reader",
            "--chan",
            "#b295",
            "--insecure",
        ])
        .env("AI_CHAT_HOME", &dir)
        .output()
        .unwrap();
    let poster_dir = client_dir(&server.home, "b295poster");
    for text in ["plain-one", "plain-two", "ping @b295reader now"] {
        run_client(
            &poster_dir,
            &[
                "send",
                "--server",
                &format!("127.0.0.1:{}", server.port),
                "--nick",
                "b295poster",
                "--chan",
                "#b295",
                "--text",
                text,
                "--insecure",
            ],
        );
    }
    let mentions = stdout_string(
        &Command::new(&binary)
            .args([
                "read",
                "--server",
                &format!("127.0.0.1:{}", server.port),
                "--nick",
                "b295reader",
                "--chan",
                "#b295",
                "--mentions",
                "--insecure",
            ])
            .env("AI_CHAT_HOME", &dir)
            .output()
            .unwrap(),
    );
    assert!(
        mentions.contains("ping @b295reader now"),
        "B295 setup: the mentions read did not return the mention: [{mentions}]"
    );
    let after = stdout_string(
        &Command::new(&binary)
            .args([
                "read",
                "--server",
                &format!("127.0.0.1:{}", server.port),
                "--nick",
                "b295reader",
                "--chan",
                "#b295",
                "--insecure",
            ])
            .env("AI_CHAT_HOME", &dir)
            .output()
            .unwrap(),
    );
    assert!(
        after.contains("plain-one"),
        "B295: a mention-filtered remote read moved the cursor past an unread plain message: [{after}]"
    );
}

// ---- mention-notify: tail --mentions --mention-exit exits on a mention ----
#[test]
fn tail_mentions_mention_exit_exits_on_a_mention_from_a_peer() {
    let server = spawn_server("e2e-mention", &[]);
    let binary = support::resolve_workspace_binary("chat-client-rs");
    let mhome = client_dir(&server.home, "ment");
    let mpeer = client_dir(&server.home, "ment-peer");
    Command::new(&binary)
        .args([
            "session",
            "set",
            "--server",
            &format!("127.0.0.1:{}", server.port),
            "--nick",
            "mwatcher",
        ])
        .env("AI_CHAT_HOME", &mhome)
        .output()
        .unwrap();
    Command::new(&binary)
        .args([
            "session",
            "set",
            "--server",
            &format!("127.0.0.1:{}", server.port),
            "--nick",
            "mpeer",
        ])
        .env("AI_CHAT_HOME", &mpeer)
        .output()
        .unwrap();
    Command::new(&binary)
        .args(["join", "--chan", "#ment", "--insecure"])
        .env("AI_CHAT_HOME", &mhome)
        .output()
        .unwrap();
    let mut ment_child = Command::new(&binary)
        .args([
            "tail",
            "--chan",
            "#ment",
            "--mentions",
            "--mention-exit",
            "--insecure",
        ])
        .env("AI_CHAT_HOME", &mhome)
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdout = ment_child.stdout.take().unwrap();
    let log = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    {
        let log = std::sync::Arc::clone(&log);
        thread::spawn(move || {
            let mut buf = [0u8; 4096];
            while let Ok(n) = stdout.read(&mut buf) {
                if n == 0 {
                    break;
                }
                log.lock()
                    .unwrap()
                    .push_str(&String::from_utf8_lossy(&buf[..n]));
            }
        });
    }
    thread::sleep(Duration::from_secs(5));
    let send_out = Command::new(&binary)
        .args([
            "send",
            "--chan",
            "#ment",
            "--text",
            "ping @mwatcher now",
            "--insecure",
        ])
        .env("AI_CHAT_HOME", &mpeer)
        .output()
        .unwrap();
    assert!(
        send_out.status.success(),
        "mention send failed: {}",
        stderr_string(&send_out)
    );
    let deadline = Instant::now() + Duration::from_secs(12);
    let mut exited = false;
    while Instant::now() < deadline {
        if matches!(ment_child.try_wait(), Ok(Some(_))) {
            exited = true;
            break;
        }
        thread::sleep(Duration::from_millis(200));
    }
    let _guard = ChildGuard(ment_child);
    assert!(
        exited,
        "tail --mentions --mention-exit did not exit on a mention"
    );
    assert!(
        log.lock().unwrap().contains("!! MENTION !!"),
        "mention was not surfaced: [{}]",
        log.lock().unwrap()
    );
}

// ---- B1xx: a dead peer must not pin a core, and must give its nick back --
#[test]
fn a_dead_peer_does_not_spin_the_server_and_frees_its_nick() {
    let server = spawn_server("e2e-spin", &[]);
    let binary = support::resolve_workspace_binary("chat-client-rs");
    let spin_home = client_dir(&server.home, "spin");
    let mut children = Vec::new();
    for i in 1..=3 {
        let child = Command::new(&binary)
            .args([
                "tail",
                "--server",
                &format!("127.0.0.1:{}", server.port),
                "--nick",
                &format!("zombie{i}"),
                "--chan",
                "#spin",
                "--insecure",
                "--no-session",
            ])
            .env("AI_CHAT_HOME", &spin_home)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        children.push(child);
    }
    thread::sleep(Duration::from_secs(4));

    let server_cpu = || -> Option<u64> {
        let output = Command::new("ps")
            .args(["-o", "time=", "-p", &server.child.pid().to_string()])
            .output()
            .ok()?;
        let text = String::from_utf8_lossy(&output.stdout);
        let cleaned = text.trim().replace('-', ":");
        if cleaned.is_empty() {
            return None;
        }
        let mut seconds: u64 = 0;
        for field in cleaned.split(':') {
            seconds = seconds * 60 + field.parse::<u64>().ok()?;
        }
        Some(seconds)
    };
    let cpu_before = server_cpu();
    // Child::kill is SIGKILL on unix (what `kill -9` was) and TerminateProcess
    // on Windows, where there is no `kill` that can address the process.
    for child in &mut children {
        let _ = child.kill();
    }
    for mut child in children {
        let _ = child.wait();
    }
    thread::sleep(Duration::from_secs(5));
    let cpu_after = server_cpu();
    if let (Some(before), Some(after)) = (cpu_before, cpu_after) {
        let burned = after.saturating_sub(before);
        assert!(
            burned < 2,
            "the server burned {burned}s of CPU in a 5s window after 3 peers were SIGKILLed (dead-peer spin)"
        );
    }

    let reclaim_dir = client_dir(&server.home, "zrec");
    let z_sent = stdout_string(&run_client(
        &reclaim_dir,
        &[
            "send",
            "--server",
            &format!("127.0.0.1:{}", server.port),
            "--nick",
            "zombie1",
            "--chan",
            "#spin",
            "--text",
            "nick reclaimed",
        ],
    ));
    assert!(
        z_sent.contains(":zombie1!zombie1@localhost PRIVMSG #spin :nick reclaimed"),
        "a dead peer never gave its nick back: [{z_sent}]"
    );
}

// ---- with the server stopped: local reads still work -----------------------
#[test]
fn local_reads_work_with_no_server_running() {
    let server = spawn_server("e2e-local", &[]);
    let binary = support::resolve_workspace_binary("chat-client-rs");
    let home = server.home.path().to_path_buf();

    // Seed a channel while the server is up.
    let seed_dir = client_dir(&server.home, "seed");
    run_client(
        &seed_dir,
        &[
            "send",
            "--server",
            &format!("127.0.0.1:{}", server.port),
            "--nick",
            "sessioner",
            "--chan",
            "#sess",
            "--text",
            "session message",
            "--insecure",
        ],
    );
    run_client(
        &seed_dir,
        &[
            "send",
            "--server",
            &format!("127.0.0.1:{}", server.port),
            "--nick",
            "sessioner",
            "--chan",
            "#sess",
            "--text",
            "second session message",
            "--insecure",
        ],
    );

    // Stop the server -- drop only the child (kills and reaps it), keeping
    // `server.home`'s ScratchDir alive: dropping the WHOLE ChatServer would
    // also delete the scratch home (and the channel log this test still
    // needs to read) via ScratchDir's own Drop.
    let ChatServer {
        home: _home_dir,
        child,
        port: _,
    } = server;
    drop(child);

    let local_out = stdout_string(
        &Command::new(&binary)
            .args([
                "read",
                "--local",
                "--chan",
                "#sess",
                "--since",
                "0",
                "--no-session",
            ])
            .env("AI_CHAT_HOME", &home)
            .output()
            .unwrap(),
    );
    assert!(
        local_out.contains("MSG #sess") && local_out.contains("session message"),
        "read --local returned nothing with the server stopped: [{local_out}]"
    );

    let bounded = stdout_string(
        &Command::new(&binary)
            .args([
                "read",
                "--local",
                "--chan",
                "#sess",
                "--since",
                "1",
                "--no-session",
            ])
            .env("AI_CHAT_HOME", &home)
            .output()
            .unwrap(),
    )
    .lines()
    .filter(|l| l.starts_with("MSG "))
    .count();
    let unbounded = stdout_string(
        &Command::new(&binary)
            .args([
                "read",
                "--local",
                "--chan",
                "#sess",
                "--since",
                "0",
                "--no-session",
            ])
            .env("AI_CHAT_HOME", &home)
            .output()
            .unwrap(),
    )
    .lines()
    .filter(|l| l.starts_with("MSG "))
    .count();
    assert!(
        bounded < unbounded,
        "read --local ignored --since (bounded={bounded} unbounded={unbounded})"
    );

    // An unknown channel is an actionable error, not a crash or silence.
    let output = Command::new(&binary)
        .args([
            "read",
            "--local",
            "--chan",
            "#nosuchchannel",
            "--no-session",
        ])
        .env("AI_CHAT_HOME", &home)
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(66),
        "read --local on a missing channel exited {:?} (want 66)",
        output.status.code()
    );
}

// ---- F6: --local must refuse a channel name the server would refuse ------
#[test]
fn local_read_refuses_a_path_traversal_channel_name() {
    let server = spawn_server("e2e-traversal", &[]);
    let binary = support::resolve_workspace_binary("chat-client-rs");
    let home = server.home.path().to_path_buf();
    std::fs::write(
        home.join("oracle.log"),
        "this file is outside the channels directory\n",
    )
    .unwrap();

    let output = Command::new(&binary)
        .args([
            "read",
            "--local",
            "--chan",
            "../oracle",
            "--since",
            "0",
            "--no-session",
        ])
        .env("AI_CHAT_HOME", &home)
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(64),
        "read --local followed a .. traversal (want 64)"
    );
    let trav_out = stdout_string(&output);
    assert!(
        !trav_out.contains("outside the channels directory"),
        "read --local printed a file outside channels/: [{trav_out}]"
    );

    let missing = Command::new(&binary)
        .args([
            "read",
            "--local",
            "--chan",
            "../nosuchfile",
            "--since",
            "0",
            "--no-session",
        ])
        .env("AI_CHAT_HOME", &home)
        .output()
        .unwrap();
    assert_eq!(
        missing.status.code(),
        output.status.code(),
        "read --local leaks whether a traversal target exists"
    );

    let tail_missing = Command::new(&binary)
        .args(["tail", "--local", "--chan", "../nosuchfile", "--no-session"])
        .env("AI_CHAT_HOME", &home)
        .output()
        .unwrap();
    assert_eq!(
        tail_missing.status.code(),
        Some(64),
        "tail --local accepted a .. traversal as a channel"
    );

    let absolute = Command::new(&binary)
        .args([
            "read",
            "--local",
            "--chan",
            home.join("oracle").to_str().unwrap(),
            "--since",
            "0",
            "--no-session",
        ])
        .env("AI_CHAT_HOME", &home)
        .output()
        .unwrap();
    assert_eq!(
        absolute.status.code(),
        Some(64),
        "read --local accepted an absolute path as a channel"
    );
}

// ---- B123: --version must print and return, not start a server -----------
#[test]
fn version_prints_and_returns_without_starting_a_server() {
    let binary = support::resolve_workspace_binary("chat-server-rs");
    let home = ScratchDir::new("e2e-b123");
    let output = Command::new(&binary)
        .arg("--version")
        .env("AI_CHAT_HOME", home.path())
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "chat-server-rs --version exited {:?} (want 0)",
        output.status.code()
    );
    assert!(
        stdout_string(&output).starts_with("chat-server-rs "),
        "chat-server-rs --version printed unexpected output: {}",
        stdout_string(&output)
    );
    assert!(
        !home.join("server.port").exists(),
        "chat-server-rs --version wrote server.port; it must not bind a port at all"
    );

    let home2 = ScratchDir::new("e2e-b123-typo");
    let output = Command::new(&binary)
        .arg("--prot")
        .env("AI_CHAT_HOME", home2.path())
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(64),
        "chat-server-rs --prot (a typo'd flag) exited {:?} (want 64)",
        output.status.code()
    );
    assert!(
        !home2.join("server.port").exists(),
        "chat-server-rs --prot wrote server.port; a malformed argument must not start the server"
    );
}
