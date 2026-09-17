// MODE: DEV
//! Three LAN-chat discovery contracts, migrated from
//! chat/tests/test-chat-resolution.sh (T145 goal 25, W136):
//!
//! 1. the server prefers the port recorded in `server.port` on every later
//!    start, an explicit argv port overrides it, and a taken session port
//!    falls back to ephemeral (server-constructed data -- chat-client-rs is
//!    not invoked at all for this contract);
//! 2. the announce beacon carries a connectable host, never bare localhost
//!    (server-constructed data -- chat-client-rs is invoked only as the
//!    receiving side of the beacon);
//! 3. the client resolves its server by ladder: explicit --server, then the
//!    session (probed), then the last-discovered cache, then a fresh UDP
//!    pass (chat-client-rs's own logic).
//!
//! Kept in ONE file per this step's own split-flag (AR-109, cycle 44): all
//! three describe one coherent discovery flow -- the server announces what
//! it prefers, and the client's ladder is exactly the logic that must
//! correctly interpret that announcement -- even though only the third
//! contract is chat-client-rs's own source. chat-server-rs's own binary is
//! spawned via `support::resolve_workspace_binary("chat-server-rs")` (a real
//! subprocess), never via `env!("CARGO_BIN_EXE_chat-server-rs")`, which does
//! not resolve across crates in this workspace (AR-113, cycle 45).

#[path = "../../chat-server-rs/tests/support/mod.rs"]
mod support;

use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

use support::{resolve_workspace_binary, run_client, ChildGuard, ScratchDir};

fn wait_port(home: &std::path::Path) -> Option<u16> {
    let port_file = home.join("server.port");
    for _ in 0..40 {
        if let Ok(text) = std::fs::read_to_string(&port_file) {
            if let Ok(port) = text.trim().parse::<u16>() {
                return Some(port);
            }
        }
        thread::sleep(Duration::from_millis(200));
    }
    None
}

/// Like `wait_port`, but for a server RESTARTED against an existing home:
/// keeps the file (which a restarted server reads to decide what port to
/// prefer -- deleting it first would erase the very state under test) and
/// instead waits for the recorded value to genuinely change away from
/// `previous`, so a stale read of the outgoing server's own still-on-disk
/// port is never mistaken for the new server's fresh write. If the port
/// never changes (the expected outcome when a restart is SUPPOSED to prefer
/// the same port), returns that unchanged value once the budget elapses.
fn wait_port_changed_from(home: &std::path::Path, previous: u16) -> Option<u16> {
    let port_file = home.join("server.port");
    let mut last = None;
    for _ in 0..40 {
        if let Ok(text) = std::fs::read_to_string(&port_file) {
            if let Ok(port) = text.trim().parse::<u16>() {
                if port != previous {
                    return Some(port);
                }
                last = Some(port);
            }
        }
        thread::sleep(Duration::from_millis(100));
    }
    last
}

/// Spawns chat-server-rs with its stderr captured in the background, so the
/// "taken port" fallback notice (or any other diagnostic) can be read back
/// without the pipe's buffer ever blocking the child.
struct ServerHandle {
    // Never read directly; held only so its Drop (kill + reap) fires when a
    // `ServerHandle` goes out of scope.
    #[allow(dead_code)]
    guard: ChildGuard,
    stderr: std::sync::Arc<std::sync::Mutex<String>>,
}

impl ServerHandle {
    fn stderr_text(&self) -> String {
        self.stderr.lock().unwrap().clone()
    }
}

fn start_server_silent(
    binary: &std::path::Path,
    home: &std::path::Path,
    extra_args: &[&str],
) -> ServerHandle {
    let mut cmd = Command::new(binary);
    cmd.env("AI_CHAT_HOME", home)
        .env("CHAT_ANNOUNCE", "0")
        .args(extra_args)
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    let mut child = cmd
        .spawn()
        .unwrap_or_else(|error| panic!("spawning chat-server-rs failed: {error}"));
    let mut stderr = child.stderr.take().unwrap();
    let buf = std::sync::Arc::new(std::sync::Mutex::new(String::new()));
    {
        let buf = std::sync::Arc::clone(&buf);
        thread::spawn(move || {
            use std::io::Read;
            let mut chunk = [0u8; 4096];
            while let Ok(n) = stderr.read(&mut chunk) {
                if n == 0 {
                    break;
                }
                buf.lock()
                    .unwrap()
                    .push_str(&String::from_utf8_lossy(&chunk[..n]));
            }
        });
    }
    ServerHandle {
        guard: ChildGuard(child),
        stderr: buf,
    }
}

// ---- 1. the session port is preferred across restarts; an explicit argv
//         port overrides; a taken session port falls back to ephemeral ----
#[test]
fn the_server_prefers_its_session_port_across_restarts_and_argv_overrides() {
    let binary = resolve_workspace_binary("chat-server-rs");
    let home = ScratchDir::new("resolution-port-pref");

    let server1 = start_server_silent(&binary, home.path(), &[]);
    let port1 = wait_port(home.path()).expect("first server did not report a port");
    drop(server1);

    // Deliberately kept, not deleted: this restart is expected to reuse the
    // SAME port already on disk (that is the whole contract), and a
    // restarted server reads this exact file to decide what to prefer --
    // deleting it would erase the very state under test. `wait_port` here
    // can return a stale-but-correct read; either way it is the value being
    // asserted against.
    let server1b = start_server_silent(&binary, home.path(), &[]);
    let port1b = wait_port(home.path()).expect("restarted server did not report a port");
    assert_eq!(
        port1b, port1,
        "restart did not prefer the session port: got {port1b}, recorded {port1}"
    );
    drop(server1b);

    // 2. an explicit argv port overrides the session. The file is kept (not
    // deleted -- same reasoning as above) and `wait_port_changed_from` waits
    // for a genuinely new value rather than trusting the first parseable
    // read, which could otherwise be the outgoing server1b's own port1.
    let override_port = port1 + 1;
    let server3 = start_server_silent(&binary, home.path(), &[&override_port.to_string()]);
    let port3 = wait_port_changed_from(home.path(), port1)
        .expect("explicit-port server did not report a port");
    assert_eq!(
        port3, override_port,
        "explicit port was overridden: got {port3}, wanted {override_port}"
    );
    // Leave server3 (holding override_port) running for the next case.

    // 3. a taken session port falls back to ephemeral. The file is kept
    // (server4 must read override_port to discover it is taken), and
    // `wait_port_changed_from` waits for the fallback's own new value.
    let server4 = start_server_silent(&binary, home.path(), &[]);
    let port4 = wait_port_changed_from(home.path(), override_port)
        .expect("fallback server did not report a port");
    assert_ne!(port4, override_port, "the taken session port was reused");
    thread::sleep(Duration::from_millis(300));
    let stderr4 = server4.stderr_text();
    assert!(
        stderr4.contains("taken"),
        "the taken-port fallback was not announced: {stderr4}"
    );

    drop(server3);
    drop(server4);
}

// ---- 2. the beacon carries a connectable host, never bare localhost ------
#[test]
fn the_beacon_carries_a_connectable_host_never_bare_localhost() {
    let server_binary = resolve_workspace_binary("chat-server-rs");
    let home = ScratchDir::new("resolution-beacon");
    let server = Command::new(&server_binary)
        .env("AI_CHAT_HOME", home.path())
        .env("CHAT_ANNOUNCE", "1")
        .env("CHAT_BCAST", "127.0.0.1")
        .env("CHAT_BEACON_PORT", "47995")
        .env("CHAT_ANNOUNCE_HOST", "203.0.113.7")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawning the beacon server failed");
    let _guard = ChildGuard(server);
    let port = wait_port(home.path()).expect("beacon server did not report a port");

    let dir = ScratchDir::new("resolution-beacon-client");
    let output = run_client(
        dir.path(),
        &[
            "discover",
            "--bcast",
            "127.0.0.1",
            "--beacon-port",
            "47995",
            "--wait",
            "3",
            "--json",
        ],
    );
    let disco = String::from_utf8_lossy(&output.stdout).into_owned();
    assert!(
        disco.contains(r#""host":"203.0.113.7""#),
        "the beacon did not carry the announced host: {disco}"
    );
    assert!(
        disco.contains(r#""name":"ai-chat/203.0.113.7""#),
        "the server name did not follow the announced host: {disco}"
    );
    assert!(
        !disco.contains("localhost"),
        "the beacon still announces localhost: {disco}"
    );
    assert!(
        disco.contains(&format!(r#""port":{port}"#)),
        "the beacon port did not parse: {disco}"
    );
}

// ---- 3. the ladder: dead session -> discovery -> healed session ----------
#[test]
fn the_client_ladder_heals_a_dead_session_via_discovery() {
    let server_binary = resolve_workspace_binary("chat-server-rs");
    let client_binary = resolve_workspace_binary("chat-client-rs");
    let ladder_beacon_port = "47996";
    let home = ScratchDir::new("resolution-ladder-server");
    let _server = Command::new(&server_binary)
        .env("AI_CHAT_HOME", home.path())
        .env("CHAT_ANNOUNCE", "1")
        .env("CHAT_BCAST", "127.0.0.1")
        .env("CHAT_BEACON_PORT", ladder_beacon_port)
        .env("CHAT_ANNOUNCE_HOST", "127.0.0.1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map(ChildGuard)
        .expect("spawning the ladder server failed");
    wait_port(home.path()).expect("ladder server did not report a port");

    let client_dir = ScratchDir::new("resolution-ladder-client");
    let session_cmd = |args: &[&str]| -> std::process::Output {
        Command::new(&client_binary)
            .args(args)
            .env("AI_CHAT_HOME", client_dir.path())
            .env("AI_CHAT_BEACON_PORT", ladder_beacon_port)
            .output()
            .expect("running chat-client-rs failed")
    };

    let set = session_cmd(&[
        "session",
        "set",
        "--server",
        "127.0.0.1:1",
        "--nick",
        "junkbox",
    ]);
    assert!(set.status.success(), "session set failed");
    let send = session_cmd(&["send", "--chan", "#ladder", "--text", "resolve me"]);
    assert!(
        send.status.success(),
        "send did not resolve a dead session to the live server: {}",
        String::from_utf8_lossy(&send.stderr)
    );

    let show = String::from_utf8_lossy(&session_cmd(&["session", "show"]).stdout).into_owned();
    let healed = show
        .lines()
        .find_map(|l| l.strip_prefix("server="))
        .unwrap_or_default()
        .to_string();
    assert_ne!(
        healed, "127.0.0.1:1",
        "the session did not heal away from the dead address: {healed}"
    );
    let read = session_cmd(&[
        "read", "--server", &healed, "--nick", "junkbox", "--chan", "#ladder", "--since", "0",
    ]);
    assert!(
        read.status.success(),
        "the healed session address is not alive: {healed}"
    );
    let cache = std::fs::read_to_string(client_dir.path().join("discovered-servers.txt"))
        .unwrap_or_default();
    assert!(
        cache.contains(&healed),
        "the resolved server was not cached: {cache}"
    );

    // The cache is the fast track: with the session pointed at a dead port
    // again, the cached address answers before any beacon is needed.
    session_cmd(&[
        "session",
        "set",
        "--server",
        "127.0.0.1:1",
        "--nick",
        "junkbox",
    ]);
    let cached_read = session_cmd(&["read", "--chan", "#ladder", "--since", "0"]);
    assert!(
        cached_read.status.success(),
        "the cache fast-track did not resolve the live server"
    );
    let show2 = String::from_utf8_lossy(&session_cmd(&["session", "show"]).stdout).into_owned();
    let healed2 = show2
        .lines()
        .find_map(|l| l.strip_prefix("server="))
        .unwrap_or_default()
        .to_string();
    assert_eq!(healed2, healed, "the cache hit did not heal the session");

    // An explicit --server wins and is used as-is: the dead address surfaces
    // in the connect error rather than being silently replaced.
    let explicit = session_cmd(&[
        "read",
        "--server",
        "127.0.0.1:1",
        "--nick",
        "junkbox",
        "--chan",
        "#ladder",
        "--since",
        "0",
    ]);
    let explicit_err = String::from_utf8_lossy(&explicit.stderr).into_owned();
    assert!(
        explicit_err.contains("127.0.0.1:1"),
        "an explicit --server did not win: {explicit_err}"
    );
}
