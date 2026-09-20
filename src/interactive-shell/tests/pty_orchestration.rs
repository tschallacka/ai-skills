// MODE: DEV
#![cfg(unix)]
//! Real PTY-driven program orchestration, migrated from
//! interactive-shell/tests/test-interactive-shell.sh (nano end to end) and
//! interactive-shell/tests/test-interactive-shell-exploration.sh (nano, mc,
//! and less through the wrapper's own socket protocol) (T145 goal 25, W137).
//!
//! The material difference from protocol.rs's own coverage (spot-checked
//! before writing this file, AR-110 cycle 44's own redundancy requirement):
//! protocol.rs already spawns interactive-shell/interactive-shell-input via
//! CARGO_BIN_EXE_ dozens of times, including real PTY plumbing, but always
//! against the synthetic `interactive-shell-fixture` binary. Every test here
//! instead drives a REAL external program (nano, mc, less) -- the fixture-
//! vs-real-program distinction, not spawning-vs-not, is what separates the
//! two files' own scope. No overlap was found: protocol.rs never spawns
//! nano/mc/less, and nothing here re-proves a fixture-driven protocol
//! contract protocol.rs already covers.
//!
//! A missing nano/mc/less (or a non-GNU nano, whose bindings this drives
//! directly -- META-RIGHT for next-word, CTRL-E for end-of-line, CTRL-O for
//! write-out, none of which macOS's stock Pico shares) is a loud SKIP, not a
//! failure, mirroring the bash originals' own posture.

use interactive_shell_core::ClientStream;
use serde_json::Value;
use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::{Mutex, PoisonError};
use std::thread;
use std::time::Duration;

const READY_POLLS: usize = 3000;
const POLL_INTERVAL: Duration = Duration::from_millis(10);

/// The wrapper refuses a socket whose parent directory is not private (0700)
/// and owned by the current user -- matching protocol.rs's own `temp_dir()`.
fn temp_dir(label: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!(
        "interactive-shell-pty-{label}-{}",
        std::process::id()
    ));
    private_dir(&path);
    path
}

/// Creates `path` (if missing) and sets it 0700 -- every directory that will
/// hold a wrapper socket needs this, not only the top-level scratch root.
fn private_dir(path: &Path) {
    fs::create_dir_all(path).unwrap();
    fs::set_permissions(path, fs::Permissions::from_mode(0o700)).unwrap();
}

/// `nano --version`'s first line names GNU nano specifically -- this drives
/// its own bindings (META-RIGHT, CTRL-E, CTRL-O), which a non-GNU nano (e.g.
/// stock macOS Pico) does not share.
fn gnu_nano_available() -> bool {
    let Ok(output) = Command::new("nano").arg("--version").output() else {
        return false;
    };
    let first_line = String::from_utf8_lossy(&output.stdout)
        .lines()
        .next()
        .unwrap_or_default()
        .to_string();
    first_line.contains("GNU nano")
}

fn tool_available(name: &str) -> bool {
    Command::new(name)
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
        || Command::new("sh")
            .arg("-c")
            .arg(format!("command -v {name}"))
            .stdout(Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false)
}

macro_rules! skip_unless {
    ($cond:expr, $why:expr) => {
        if !$cond {
            eprintln!("SKIP: {}", $why);
            return;
        }
    };
}

fn start_real_program(
    dir: &Path,
    cwd: Option<&Path>,
    command: &[&str],
    idle: &str,
) -> std::process::Child {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_interactive-shell"));
    cmd.args([
        "--socket",
        dir.join("socket").to_str().unwrap(),
        "--cols",
        "80",
        "--rows",
        "24",
        "--idle-timeout",
        idle,
        "--",
    ])
    .args(command)
    // Never Stdio::piped() here without a background thread draining it: the
    // wrapper's own JSONL protocol stream writes to its stdout regardless of
    // whether a caller is watching, so an unread pipe fills its OS buffer and
    // the wrapper's next write() blocks forever -- including its own control
    // loop, so it then stops answering the socket too. Nothing here needs
    // that stream (only the socket protocol is used), so null it outright.
    .stdout(Stdio::null())
    .stderr(Stdio::null());
    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }
    cmd.spawn().expect("spawning interactive-shell failed")
}

fn wait_for_socket(dir: &Path) {
    for _ in 0..READY_POLLS {
        if dir.join("socket").exists() {
            return;
        }
        thread::sleep(POLL_INTERVAL);
    }
    panic!("socket did not appear at {}", dir.join("socket").display());
}

/// `connect_in_directory` reaches the socket by NAME from inside its own
/// directory. An absolute `UnixStream::connect` overflows sun_path (104 bytes
/// on macOS, 108 on Linux) under the long scratch directory run-tests.sh
/// hands every test -- the failure `could not connect ... path must be
/// shorter than SUN_LEN`, with the socket present the whole time. The move
/// is an fchdir, and the cwd is per PROCESS while the two tests here run as
/// threads, so it is serialised exactly as protocol.rs does.
static CWD_LOCK: Mutex<()> = Mutex::new(());

fn connect(dir: &Path) -> ClientStream {
    let socket = dir.join("socket");
    let mut last_error = String::new();
    for _ in 0..READY_POLLS {
        let attempt = {
            let _guard = CWD_LOCK.lock().unwrap_or_else(PoisonError::into_inner);
            interactive_shell_core::connect_in_directory(&socket)
        };
        match attempt {
            Ok(stream) => return stream,
            Err(error) => last_error = error,
        }
        thread::sleep(POLL_INTERVAL);
    }
    panic!("could not connect to {}: {last_error}", socket.display());
}

fn request_all(dir: &Path, body: &str) -> Vec<Value> {
    let mut stream = connect(dir);
    stream.write_all(body.as_bytes()).unwrap();
    stream.shutdown(std::net::Shutdown::Write).unwrap();
    let mut out = String::new();
    stream.read_to_string(&mut out).unwrap();
    out.lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| serde_json::from_str(l).unwrap())
        .collect()
}

fn request(dir: &Path, body: &str) -> Value {
    request_all(dir, body).into_iter().next().unwrap()
}

/// Poll `observe` until the screen carries `needle`.
fn wait_for_rows(dir: &Path, needle: &str) -> Value {
    let mut last = Value::Null;
    for _ in 0..READY_POLLS {
        if let Some(snapshot) = request_all(dir, "{\"v\":1,\"op\":\"observe\"}\n")
            .into_iter()
            .find(|event| event["event"] == "snapshot")
        {
            if snapshot["rows"].to_string().contains(needle) {
                return snapshot;
            }
            last = snapshot;
        }
        thread::sleep(POLL_INTERVAL);
    }
    panic!(
        "{needle:?} never reached the screen; last snapshot rows: {}",
        last["rows"]
    );
}

fn wait_for_rows_any(dir: &Path, needles: &[&str]) -> Value {
    let mut last = Value::Null;
    for _ in 0..READY_POLLS {
        if let Some(snapshot) = request_all(dir, "{\"v\":1,\"op\":\"observe\"}\n")
            .into_iter()
            .find(|event| event["event"] == "snapshot")
        {
            let rows = snapshot["rows"].to_string();
            if needles.iter().any(|n| rows.contains(n)) {
                return snapshot;
            }
            last = snapshot;
        }
        thread::sleep(POLL_INTERVAL);
    }
    panic!(
        "none of {needles:?} ever reached the screen; last snapshot rows: {}",
        last["rows"]
    );
}

fn send_text(dir: &Path, text: &str) {
    let body = serde_json::json!({"v":1,"op":"text","text":text}).to_string() + "\n";
    let _ = request(dir, &body);
}

fn send_key(dir: &Path, key: &str) {
    let body = serde_json::json!({"v":1,"op":"key","key":key}).to_string() + "\n";
    let _ = request(dir, &body);
}

fn send_raw(dir: &Path, hex: &str) {
    let body = serde_json::json!({"v":1,"op":"raw","hex":hex}).to_string() + "\n";
    let _ = request(dir, &body);
}

fn shutdown(dir: &Path) {
    let _ = request(dir, "{\"v\":1,\"op\":\"shutdown\"}\n");
}

fn read_file_hex(path: &Path) -> String {
    let bytes = fs::read(path).unwrap_or_default();
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

// ---- nano end to end, driven through the PTY wrapper ----------------------
#[test]
fn nano_edits_a_real_file_through_the_pty_wrapper() {
    skip_unless!(
        gnu_nano_available(),
        "nano is unavailable or is not GNU nano"
    );

    let dir = temp_dir("nano-flow");
    let target_name = "test.txt";
    let target = dir.join(target_name);

    let mut child = start_real_program(
        &dir,
        Some(&dir),
        &["nano", "--ignorercfiles", target_name],
        "20",
    );
    wait_for_socket(&dir);
    wait_for_rows(&dir, "GNU nano");
    send_text(&dir, "Hello World");
    send_key(&dir, "CTRL-X");
    wait_for_rows(&dir, "Save modified buffer");
    send_raw(&dir, "59"); // 'Y'
    wait_for_rows_any(&dir, &["Write to File", "File Name to Write"]);
    send_key(&dir, "ENTER");
    // Wait for the wrapper's own lifecycle event (the child exiting after save).
    let _ = request_all(&dir, "{\"v\":1,\"op\":\"observe\"}\n");
    let status = child.wait().expect("waiting on the wrapper failed");
    assert!(
        status.success(),
        "the wrapper did not exit cleanly: {status:?}"
    );
    assert_eq!(
        read_file_hex(&target),
        "48656c6c6f20576f726c640a",
        "nano did not save the expected content"
    );
}

// ---- nano/mc/less observation-driven exploration ---------------------------
#[test]
fn observation_driven_exploration_across_nano_mc_and_less() {
    skip_unless!(
        tool_available("mc") && gnu_nano_available() && tool_available("less"),
        "mc, nano (GNU), and less are required"
    );

    let run_dir = temp_dir("exploration");
    let mc_dir = run_dir.join("files");
    fs::create_dir_all(&mc_dir).unwrap();
    let target = mc_dir.join("test.txt");

    // ---- nano: open, type, save ----
    let nano_dir = run_dir.join("nano-sock");
    private_dir(&nano_dir);
    let mut nano_child = Command::new(env!("CARGO_BIN_EXE_interactive-shell"))
        .args([
            "--socket",
            nano_dir.join("socket").to_str().unwrap(),
            "--cols",
            "100",
            "--rows",
            "30",
            "--idle-timeout",
            "30",
            "--",
            "nano",
            "--ignorercfiles",
            target.to_str().unwrap(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawning the nano wrapper failed");
    wait_for_socket(&nano_dir);
    wait_for_rows(&nano_dir, "test.txt");
    send_text(&nano_dir, "Hello World");
    send_key(&nano_dir, "CTRL-X");
    wait_for_rows(&nano_dir, "Save modified buffer");
    send_raw(&nano_dir, "59");
    wait_for_rows(&nano_dir, "Write to File:");
    send_key(&nano_dir, "ENTER");
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while std::time::Instant::now() < deadline
        && !fs::read_to_string(&target)
            .unwrap_or_default()
            .contains("Hello World")
    {
        thread::sleep(Duration::from_millis(100));
    }
    assert!(
        fs::read_to_string(&target)
            .unwrap_or_default()
            .contains("Hello World"),
        "nano never wrote the expected content"
    );
    let _ = nano_child.wait();

    // ---- mc: navigate, open the file via F4 (EDITOR=nano), edit, save ----
    let mc_dir_sock = run_dir.join("mc-sock");
    private_dir(&mc_dir_sock);
    let mut mc_child = Command::new(env!("CARGO_BIN_EXE_interactive-shell"))
        .env("EDITOR", "nano")
        .args([
            "--socket",
            mc_dir_sock.join("socket").to_str().unwrap(),
            "--cols",
            "120",
            "--rows",
            "35",
            "--idle-timeout",
            "60",
            "--",
            "mc",
            "-u",
            mc_dir.to_str().unwrap(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawning the mc wrapper failed");
    wait_for_socket(&mc_dir_sock);
    wait_for_rows(&mc_dir_sock, "test.txt");

    let target_name = "test.txt";
    let mut found = false;
    for _ in 0..40 {
        let snapshot = request_all(&mc_dir_sock, "{\"v\":1,\"op\":\"observe\"}\n")
            .into_iter()
            .find(|e| e["event"] == "snapshot")
            .unwrap_or(Value::Null);
        if let Some(elements) = snapshot["elements"].as_array() {
            if let Some(element) = elements
                .iter()
                .find(|e| e["label"].as_str() == Some(target_name))
            {
                let row = element["row"].as_u64().unwrap_or(0) + 1;
                let col = element["col"].as_u64().unwrap_or(0) + 1;
                let click = serde_json::json!({"v":1,"op":"click","x":col,"y":row,"button":0})
                    .to_string()
                    + "\n";
                let _ = request(&mc_dir_sock, &click);
                found = true;
                break;
            }
        }
        send_key(&mc_dir_sock, "PAGEDOWN");
        thread::sleep(Duration::from_millis(100));
    }
    assert!(
        found,
        "the exploration never located {target_name} in mc's own pane"
    );

    send_key(&mc_dir_sock, "F4");
    wait_for_rows(&mc_dir_sock, "Hello World");
    // B125-verified live (2026-09-10, per the migrated bash comment): F4
    // opens the $EDITOR-named program (nano here) on the discovered file,
    // not mc's own built-in mcedit.
    send_text(&mc_dir_sock, " EDITED");
    send_key(&mc_dir_sock, "CTRL-O");
    wait_for_rows(&mc_dir_sock, "Write to File:");
    send_key(&mc_dir_sock, "ENTER");
    let deadline = std::time::Instant::now() + Duration::from_secs(10);
    while std::time::Instant::now() < deadline
        && !fs::read_to_string(&target)
            .unwrap_or_default()
            .contains("EDITED")
    {
        thread::sleep(Duration::from_millis(100));
    }
    assert!(
        fs::read_to_string(&target)
            .unwrap_or_default()
            .contains("EDITED"),
        "the mc-driven nano edit was never saved"
    );
    send_key(&mc_dir_sock, "CTRL-X");
    shutdown(&mc_dir_sock);
    let _ = mc_child.wait();

    // ---- less: a file whose content differs from what nano wrote, written
    // directly (the wrapper-driven edit path is already proven above) ----
    fs::write(&target, "hello universe\n").unwrap();
    let less_dir = run_dir.join("less-sock");
    private_dir(&less_dir);
    let mut less_child = Command::new(env!("CARGO_BIN_EXE_interactive-shell"))
        .args([
            "--socket",
            less_dir.join("socket").to_str().unwrap(),
            "--cols",
            "100",
            "--rows",
            "30",
            "--idle-timeout",
            "30",
            "--",
            "less",
            target.to_str().unwrap(),
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawning the less wrapper failed");
    wait_for_socket(&less_dir);
    wait_for_rows(&less_dir, "hello universe");
    send_key(&less_dir, "q");
    let _ = less_child.wait();
}
