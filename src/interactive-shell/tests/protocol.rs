// MODE: DEV
use serde_json::Value;
use std::fs;
use std::fs::OpenOptions;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

/// How many 10ms polls a readiness wait is allowed: 3000, so thirty seconds.
///
/// These loops were `0..100`, a ONE second budget, which is ample on a Linux
/// runner and far too tight on the macOS one. In run 33793295763, 17 of 19
/// tests here failed on it while the binary and its socket were both fine:
/// `signal_cleanup_removes_socket` created and removed a socket successfully
/// and `malformed_cli_arguments_do_not_panic` ran the CLI, so neither bind nor
/// the executable was at fault. The macOS runner is a shared, oversubscribed
/// VPS that pauses for other tenants, so a readiness budget has to cover the
/// worst scheduling delay rather than the typical one.
///
/// It is a CEILING, not a sleep: every loop returns the moment its condition
/// holds, so a healthy run is no slower than it was. Only a genuine failure
/// pays the thirty seconds, and it pays it once.
const READY_POLLS: usize = 3000;

/// The gap between polls. Kept small so readiness is detected promptly; the
/// budget above is what bounds the wait.
const POLL_INTERVAL: Duration = Duration::from_millis(10);

/// Where a wrapper's stderr is kept, inside the test's own directory.
///
/// It was `Stdio::null()`. That is why a wrapper that failed before it could
/// bind produced seventeen "socket did not appear" panics on the macOS runner
/// and not one line saying what went wrong -- the diagnosis had to be guessed
/// at from which tests failed. A FILE and not a pipe, because the wrapper
/// outlives the assertion and a pipe nobody drains blocks it once the buffer
/// fills; opened for append so several wrappers in one directory accumulate
/// rather than truncating each other.
fn stderr_path(dir: &Path) -> PathBuf {
    dir.join("wrapper.stderr")
}

/// Whatever the wrapper said, ready to append to a panic message.
fn wrapper_stderr(dir: &Path) -> String {
    match fs::read(stderr_path(dir)) {
        Ok(bytes) if !bytes.is_empty() => {
            format!(
                "wrapper stderr:\n{}",
                String::from_utf8_lossy(&bytes).trim_end()
            )
        }
        _ => "wrapper stderr: (empty)".to_string(),
    }
}

fn temp_dir(label: &str) -> PathBuf {
    let path =
        std::env::temp_dir().join(format!("interactive-shell-{label}-{}", std::process::id()));
    fs::create_dir_all(&path).unwrap();
    fs::set_permissions(&path, fs::Permissions::from_mode(0o700)).unwrap();
    path
}

fn start(dir: &Path, command: &[&str], idle: &str) -> Child {
    start_binary(env!("CARGO_BIN_EXE_interactive-shell"), dir, command, idle)
}

fn start_binary(binary: &str, dir: &Path, command: &[&str], idle: &str) -> Child {
    Command::new(binary)
        .args([
            "--socket",
            dir.join("socket").to_str().unwrap(),
            "--cols",
            "20",
            "--rows",
            "4",
            "--idle-timeout",
            idle,
            "--",
        ])
        .args(command)
        .stdout(Stdio::piped())
        .stderr(Stdio::from(
            OpenOptions::new()
                .create(true)
                .append(true)
                .open(stderr_path(dir))
                .unwrap(),
        ))
        .spawn()
        .unwrap()
}

fn start_fixture(dir: &Path, idle: &str) -> Child {
    start_binary(
        env!("CARGO_BIN_EXE_interactive-shell"),
        dir,
        &[env!("CARGO_BIN_EXE_interactive-shell-fixture")],
        idle,
    )
}

fn request(dir: &Path, body: &str) -> Value {
    for _ in 0..READY_POLLS {
        if let Ok(mut stream) = UnixStream::connect(dir.join("socket")) {
            stream.write_all(body.as_bytes()).unwrap();
            stream.shutdown(std::net::Shutdown::Write).unwrap();
            let mut out = String::new();
            stream.read_to_string(&mut out).unwrap();
            return serde_json::from_str(out.trim()).unwrap();
        }
        thread::sleep(POLL_INTERVAL);
    }
    panic!(
        "socket did not appear at {} within {:?}\n{}",
        dir.join("socket").display(),
        POLL_INTERVAL * READY_POLLS as u32,
        wrapper_stderr(dir)
    )
}

fn request_all(dir: &Path, body: &str) -> Vec<Value> {
    for _ in 0..READY_POLLS {
        if let Ok(mut stream) = UnixStream::connect(dir.join("socket")) {
            stream.write_all(body.as_bytes()).unwrap();
            stream.shutdown(std::net::Shutdown::Write).unwrap();
            let mut out = String::new();
            stream.read_to_string(&mut out).unwrap();
            return out
                .lines()
                .map(|line| serde_json::from_str(line).unwrap())
                .collect();
        }
        thread::sleep(POLL_INTERVAL);
    }
    panic!(
        "socket did not appear at {} within {:?}\n{}",
        dir.join("socket").display(),
        POLL_INTERVAL * READY_POLLS as u32,
        wrapper_stderr(dir)
    )
}

#[test]
fn wrapper_reports_screen_ack_and_lifecycle() {
    let dir = temp_dir("events");
    let mut child = start(&dir, &["sh", "-c", "stty size; printf first; sleep 1"], "5");
    let ack = request(
        &dir,
        r#"{"v":1,"op":"key","key":"ENTER"}
"#,
    );
    assert_eq!(ack["event"], "ack");
    let mut output = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut output)
        .unwrap();
    let status = child.wait().unwrap();
    assert!(status.success());
    assert!(output
        .lines()
        .any(|line| line.contains("\"event\":\"screen\"")));
    assert!(output
        .lines()
        .any(|line| line.contains("\"event\":\"lifecycle\"")));
    assert!(!dir.join("socket").exists());
}

#[test]
fn screen_deltas_chain_and_publish_restored_primary_rows() {
    let dir = temp_dir("alternate-event");
    let mut child = start(
        &dir,
        &[
            "sh",
            "-c",
            "printf primary; sleep .1; printf '\\x1b[?1049hALT\\x1b[?1049l'; sleep .2",
        ],
        "5",
    );
    let mut output = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut output)
        .unwrap();
    child.wait().unwrap();
    let events: Vec<Value> = output
        .lines()
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    let screens: Vec<&Value> = events.iter().filter(|v| v["event"] == "screen").collect();
    assert!(
        !screens.is_empty(),
        "no screen events\n{}",
        wrapper_stderr(&dir)
    );
    assert_eq!(screens[0]["seq"], 1);
    assert_eq!(screens[0]["base"], 0);
    for pair in screens.windows(2) {
        assert_eq!(pair[1]["base"].as_u64(), pair[0]["seq"].as_u64());
    }
    assert!(screens
        .iter()
        .any(|v| v["rows"].to_string().contains("primary")));
    assert!(screens.last().unwrap()["rows"]
        .to_string()
        .contains("primary"));
}

#[test]
fn protocol_observes_fragmented_osc_overflow_without_leaking_payload() {
    let dir = temp_dir("fragmented-osc");
    let mut child = start_fixture(&dir, "5");
    let mut output = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut output)
        .unwrap();
    child.wait().unwrap();
    let rows = output
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|event| event["event"] == "screen")
        .flat_map(|event| {
            event["rows"]
                .as_object()
                .map(|rows| {
                    rows.values()
                        .filter_map(|row| row.as_str().map(str::to_owned))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        rows.contains("CSI_SAFE"),
        "CSI_SAFE never reached the screen\n{}",
        wrapper_stderr(&dir)
    );
    assert!(rows.contains("OSC_SAFE"));
    assert!(!rows.contains(&"1".repeat(129)));
    assert!(!rows.contains(&"x".repeat(4097)));
}

#[test]
fn socket_is_private_and_invalid_requests_are_rejected() {
    let dir = temp_dir("bounds");
    let mut child = start(&dir, &["sleep", "2"], "5");
    for _ in 0..READY_POLLS {
        if dir.join("socket").exists() {
            break;
        }
        thread::sleep(POLL_INTERVAL);
    }
    let mode = fs::metadata(dir.join("socket"))
        .unwrap_or_else(|e| panic!("no socket to inspect: {e}\n{}", wrapper_stderr(&dir)))
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(mode, 0o600);
    let error = request(
        &dir,
        r#"{"v":1,"op":"raw","hex":"0"}
"#,
    );
    assert_eq!(error["event"], "error");
    let malformed = request(&dir, "{\"v\":1,\"op\":\"text\"}\n");
    assert_eq!(malformed["event"], "error");
    let combination = Command::new(env!("CARGO_BIN_EXE_interactive-shell-input"))
        .args([
            "--socket",
            dir.join("socket").to_str().unwrap(),
            "key",
            "x",
            "shutdown",
        ])
        .output()
        .unwrap();
    assert!(!combination.status.success());
    let _ = request(
        &dir,
        r#"{"v":1,"op":"shutdown"}
"#,
    );
    child.wait().unwrap();
    assert!(!dir.join("socket").exists());
}

#[test]
fn closed_output_removes_socket_before_exit() {
    let dir = temp_dir("closed-output");
    let mut child = start(&dir, &["yes"], "30");
    for _ in 0..READY_POLLS {
        if dir.join("socket").exists() {
            break;
        }
        thread::sleep(POLL_INTERVAL);
    }
    assert!(
        dir.join("socket").exists(),
        "the socket was never created\n{}",
        wrapper_stderr(&dir)
    );
    drop(child.stdout.take());
    let status = child.wait().unwrap();
    assert!(!status.success());
    assert!(!dir.join("socket").exists());
}

#[test]
fn invalid_dimensions_fail_before_creating_socket() {
    let dir = temp_dir("dimensions");
    let output = Command::new(env!("CARGO_BIN_EXE_interactive-shell"))
        .args([
            "--socket",
            dir.join("socket").to_str().unwrap(),
            "--cols",
            "0",
            "--rows",
            "4",
            "--",
            "true",
        ])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(!dir.join("socket").exists());
    let upper = Command::new(env!("CARGO_BIN_EXE_interactive-shell"))
        .args([
            "--socket",
            dir.join("upper").to_str().unwrap(),
            "--cols",
            "240",
            "--rows",
            "100",
            "--idle-timeout",
            "1",
            "--",
            "true",
        ])
        .output()
        .unwrap();
    assert!(
        upper.status.success(),
        "the upper-bound wrapper failed: {}",
        String::from_utf8_lossy(&upper.stderr).trim_end()
    );
    for (option, value) in [("--rows", "0"), ("--cols", "241"), ("--rows", "101")] {
        let output = Command::new(env!("CARGO_BIN_EXE_interactive-shell"))
            .args([
                "--socket",
                dir.join(format!("{option}-{value}")).to_str().unwrap(),
                option,
                value,
                "--",
                "true",
            ])
            .output()
            .unwrap();
        assert!(!output.status.success());
    }
}

#[test]
fn structured_observe_paste_mouse_and_resize_requests_work() {
    let dir = temp_dir("structured-actions");
    let mut child = start(&dir, &["sh", "-c", "sleep 2"], "5");
    let observed = request_all(
        &dir,
        r#"{"v":1,"op":"observe"}
"#,
    );
    assert_eq!(observed[0]["event"], "snapshot");
    assert_eq!(observed[1]["event"], "ack");
    assert_eq!(
        request(
            &dir,
            r#"{"v":1,"op":"paste","text":"pasted"}
"#
        )["event"],
        "ack"
    );
    assert_eq!(
        request(
            &dir,
            r#"{"v":1,"op":"mouse","x":2,"y":2,"button":0,"action":"down"}
"#
        )["event"],
        "ack"
    );
    assert_eq!(
        request(
            &dir,
            r#"{"v":1,"op":"resize","cols":30,"rows":6}
"#
        )["event"],
        "ack"
    );
    let _ = request(
        &dir,
        r#"{"v":1,"op":"shutdown"}
"#,
    );
    child.wait().unwrap();
}

#[test]
fn view_is_compact_numbered_and_supports_rows_and_deltas() {
    let dir = temp_dir("view");
    let mut child = start(
        &dir,
        &["sh", "-c", "printf ONE; sleep .1; printf '\\rTWO'; sleep 1"],
        "5",
    );
    let first_wait = request_all(
        &dir,
        r#"{"v":1,"op":"wait","contains":"ONE"}
"#,
    );
    assert!(first_wait.iter().any(|event| event["matched"] == true));
    let view = Command::new(env!("CARGO_BIN_EXE_interactive-shell-input"))
        .args([
            "--socket",
            dir.join("socket").to_str().unwrap(),
            "view",
            "1",
        ])
        .output()
        .unwrap();
    assert!(view.status.success());
    assert_eq!(
        String::from_utf8(view.stdout).unwrap(),
        "terminal=20x4\n001 [001-003] ONE\n"
    );
    let range = Command::new(env!("CARGO_BIN_EXE_interactive-shell-input"))
        .args([
            "--socket",
            dir.join("socket").to_str().unwrap(),
            "view",
            "1-2",
        ])
        .output()
        .unwrap();
    assert!(range.status.success());
    assert!(String::from_utf8(range.stdout)
        .unwrap()
        .starts_with("terminal=20x4\n001 [001-003] ONE\n002 "));
    let second_wait = request_all(
        &dir,
        r#"{"v":1,"op":"wait","contains":"TWO"}
"#,
    );
    assert!(second_wait.iter().any(|event| event["matched"] == true));
    let delta = request_all(
        &dir,
        r#"{"v":1,"op":"view-delta","rows":[1]}
"#,
    );
    assert_eq!(delta[0]["event"], "view");
    assert_eq!(delta[0]["text"], "001 [001-003] TWO");
    let _ = request(
        &dir,
        r#"{"v":1,"op":"shutdown"}
"#,
    );
    child.wait().unwrap();
}

#[test]
fn rgbview_preserves_styles_without_json_wrapping() {
    let dir = temp_dir("rgbview");
    let mut child = start(
        &dir,
        &["sh", "-c", "printf '\\033[31mRED\\033[0m'; sleep 1"],
        "5",
    );
    let _ = request_all(
        &dir,
        r#"{"v":1,"op":"wait","contains":"RED"}
"#,
    );
    let output = Command::new(env!("CARGO_BIN_EXE_interactive-shell-input"))
        .args([
            "--socket",
            dir.join("socket").to_str().unwrap(),
            "rgbview",
            "1",
        ])
        .output()
        .unwrap();
    assert!(output.status.success());
    let text = String::from_utf8(output.stdout).unwrap();
    assert!(text.contains("\x1b[0;31mRED"));
    assert!(!text.contains("\"event\""));
    let _ = request(
        &dir,
        r#"{"v":1,"op":"shutdown"}
"#,
    );
    child.wait().unwrap();
}

#[test]
fn locate_returns_visible_coordinates_without_sending_input() {
    let dir = temp_dir("locate");
    let mut child = start(&dir, &["sh", "-c", "printf TARGET; sleep 1"], "5");
    let _ = request_all(
        &dir,
        r#"{"v":1,"op":"wait","contains":"TARGET"}
"#,
    );
    let matches = request_all(
        &dir,
        r#"{"v":1,"op":"locate","query":"TARGET"}
"#,
    );
    assert_eq!(matches[0]["event"], "locate");
    assert_eq!(matches[0]["matches"][0]["row"], 1);
    assert_eq!(matches[0]["matches"][0]["col"], 1);
    assert_eq!(matches[0]["matches"][0]["width"], 6);
    assert_eq!(matches[1]["event"], "ack");
    let _ = request(
        &dir,
        r#"{"v":1,"op":"shutdown"}
"#,
    );
    child.wait().unwrap();
}

#[test]
fn wait_returns_a_snapshot_after_a_screen_predicate() {
    let dir = temp_dir("wait");
    let mut child = start(
        &dir,
        &[
            "sh",
            "-c",
            "printf before; sleep .1; printf TARGET; sleep .2",
        ],
        "5",
    );
    let events = request_all(
        &dir,
        r#"{"v":1,"op":"wait","contains":"TARGET","timeout_ms":2000}
"#,
    );
    let wait = events
        .iter()
        .find(|event| event["event"] == "wait")
        .unwrap();
    assert_eq!(wait["matched"], true);
    assert!(wait["rows"].to_string().contains("TARGET"));
    assert!(events.iter().any(|event| event["event"] == "ack"));
    child.kill().ok();
    child.wait().unwrap();
}

#[test]
fn session_file_reuses_socket_and_command_without_repeating_arguments() {
    let state = temp_dir("session-state");
    let mut child = Command::new(env!("CARGO_BIN_EXE_interactive-shell"))
        .env("INTERACTIVE_SHELL_HOME", &state)
        .env("INTERACTIVE_SHELL_AGENT", "session-agent")
        .args([
            "--session",
            "resume-case",
            "--cols",
            "20",
            "--rows",
            "4",
            "--idle-timeout",
            "5",
            "--",
            "sh",
            "-c",
            "printf SESSION_READY; sleep 5",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let session_file = state.join("sessions/resume-case.json");
    for _ in 0..READY_POLLS {
        if session_file.exists() {
            break;
        }
        thread::sleep(POLL_INTERVAL);
    }
    assert!(session_file.exists());
    let session: Value = serde_json::from_str(&fs::read_to_string(&session_file).unwrap()).unwrap();
    let socket = PathBuf::from(session["socket"].as_str().unwrap());
    assert!(socket.ends_with("term.sock"));
    assert_eq!(
        fs::metadata(&session_file).unwrap().permissions().mode() & 0o777,
        0o600
    );
    assert_eq!(
        fs::metadata(socket.parent().unwrap())
            .unwrap()
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    assert_eq!(session["agent"], "session-agent");
    let input = Command::new(env!("CARGO_BIN_EXE_interactive-shell-input"))
        .env("INTERACTIVE_SHELL_HOME", &state)
        .args(["--session", "resume-case", "wait", "SESSION_READY", "2000"])
        .output()
        .unwrap();
    assert!(
        input.status.success(),
        "session wait failed: {}\n{}",
        String::from_utf8_lossy(&input.stderr).trim_end(),
        wrapper_stderr(socket.parent().unwrap())
    );
    assert!(String::from_utf8_lossy(&input.stdout).contains("\"matched\":true"));
    let shutdown = Command::new(env!("CARGO_BIN_EXE_interactive-shell-input"))
        .env("INTERACTIVE_SHELL_HOME", &state)
        .args(["--session", "resume-case", "shutdown"])
        .output()
        .unwrap();
    assert!(shutdown.status.success());
    child.wait().unwrap();

    let mut restarted = Command::new(env!("CARGO_BIN_EXE_interactive-shell"))
        .env("INTERACTIVE_SHELL_HOME", &state)
        .args(["--session", "resume-case"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    for _ in 0..READY_POLLS {
        if socket.exists() {
            break;
        }
        thread::sleep(POLL_INTERVAL);
    }
    assert!(socket.exists());
    let resumed = Command::new(env!("CARGO_BIN_EXE_interactive-shell-input"))
        .env("INTERACTIVE_SHELL_HOME", &state)
        .args(["--session", "resume-case", "wait", "SESSION_READY", "2000"])
        .output()
        .unwrap();
    assert!(resumed.status.success());
    assert!(String::from_utf8_lossy(&resumed.stdout).contains("\"matched\":true"));
    let _ = Command::new(env!("CARGO_BIN_EXE_interactive-shell-input"))
        .env("INTERACTIVE_SHELL_HOME", &state)
        .args(["--session", "resume-case", "shutdown"])
        .output();
    restarted.wait().unwrap();
}

#[test]
fn observe_exposes_osc8_elements_and_click_targets() {
    let dir = temp_dir("elements");
    let mut child = start(
        &dir,
        &[
            "sh",
            "-c",
            "printf '\\033]8;;https://example.test\\033\\\\LINK\\033]8;;\\033\\\\'; sleep 2",
        ],
        "5",
    );
    let snapshot = request_all(
        &dir,
        r#"{"v":1,"op":"observe"}
"#,
    );
    let screen = snapshot
        .iter()
        .find(|event| event["event"] == "snapshot")
        .unwrap();
    let element = &screen["elements"][0];
    assert_eq!(element["label"], "LINK");
    assert_eq!(element["uri"], "https://example.test");
    assert_eq!(element["actionable"], true);
    assert_eq!(element["highlighted"], false);
    assert!(screen["elements"]
        .as_array()
        .unwrap()
        .iter()
        .any(|candidate| candidate["id"] == "text-0-0"));
    assert_eq!(
        screen["elements"]
            .as_array()
            .unwrap()
            .iter()
            .find(|candidate| candidate["id"] == "text-0-0")
            .unwrap()["actionable"],
        false
    );
    let elements = request_all(
        &dir,
        r#"{"v":1,"op":"elements"}
"#,
    );
    assert_eq!(elements[0]["event"], "elements");
    assert_eq!(elements[0]["elements"].as_array().unwrap().len(), 1);
    assert_eq!(elements[1]["event"], "ack");
    assert_eq!(elements[0]["elements"][0]["label"], "LINK");
    let filtered = request_all(
        &dir,
        r#"{"v":1,"op":"elements","rows":[1]}
"#,
    );
    assert_eq!(filtered[0]["event"], "elements");
    assert_eq!(filtered[0]["elements"].as_array().unwrap().len(), 1);
    let empty = request_all(
        &dir,
        r#"{"v":1,"op":"elements","rows":[2]}
"#,
    );
    assert!(empty[0]["elements"].as_array().unwrap().is_empty());
    let id = element["id"].as_str().unwrap();
    let body = format!(
        r#"{{"v":1,"op":"click","id":"{id}","button":0}}
"#
    );
    assert_eq!(request(&dir, &body)["event"], "ack");
    let _ = request(
        &dir,
        r#"{"v":1,"op":"shutdown"}
"#,
    );
    child.wait().unwrap();
}

#[test]
fn observe_drops_osc8_elements_after_their_cells_are_erased() {
    let dir = temp_dir("stale-elements");
    let mut child = start(
        &dir,
        &[
            "sh",
            "-c",
            "printf '\\033]8;;https://example.test\\033\\\\LINK\\033]8;;\\033\\\\'; sleep 0.2; printf '\\033[2J'; sleep 2",
        ],
        "5",
    );
    thread::sleep(Duration::from_millis(500));
    let snapshot = request_all(
        &dir,
        r#"{"v":1,"op":"observe"}
"#,
    )
    .into_iter()
    .find(|event| event["event"] == "snapshot")
    .unwrap();
    assert!(snapshot["elements"]
        .as_array()
        .unwrap()
        .iter()
        .all(|element| element["uri"] != "https://example.test"));
    let _ = request(
        &dir,
        r#"{"v":1,"op":"shutdown"}
"#,
    );
    child.wait().unwrap();
}

#[test]
fn malformed_cli_arguments_do_not_panic() {
    let wrapper_help = Command::new(env!("CARGO_BIN_EXE_interactive-shell"))
        .arg("--help")
        .output()
        .unwrap();
    assert!(wrapper_help.status.success());
    let wrapper_text = String::from_utf8_lossy(&wrapper_help.stdout);
    assert!(wrapper_text.contains("usage:"));
    assert!(wrapper_text.contains("--session ID"));
    assert!(wrapper_text.contains("default 80x24"));
    assert!(wrapper_text.contains("does not know application keybindings"));
    let wrapper = Command::new(env!("CARGO_BIN_EXE_interactive-shell"))
        .arg("--socket")
        .output()
        .unwrap();
    assert!(!wrapper.status.success());
    assert!(!String::from_utf8_lossy(&wrapper.stderr).contains("panicked"));
    let input = Command::new(env!("CARGO_BIN_EXE_interactive-shell-input"))
        .arg("--socket")
        .output()
        .unwrap();
    assert!(!input.status.success());
    assert!(!String::from_utf8_lossy(&input.stderr).contains("panicked"));
}

#[test]
fn cli_text_preserves_spaces_and_input_help_is_available() {
    let dir = temp_dir("cli-text");
    let mut child = start(&dir, &["sh", "-c", "sleep 3"], "5");
    let help = Command::new(env!("CARGO_BIN_EXE_interactive-shell-input"))
        .args(["--help"])
        .output()
        .unwrap();
    assert!(help.status.success());
    let help_text = String::from_utf8_lossy(&help.stdout);
    assert!(help_text.contains("operations:"));
    assert!(help_text.contains("view 10-15"));
    assert!(help_text.contains("locate TEXT"));
    assert!(help_text.contains("rgbview"));
    assert!(help_text.contains("elements [ROWS...]"));
    assert!(help_text.contains("Application keybindings are unknown"));
    let input = Command::new(env!("CARGO_BIN_EXE_interactive-shell-input"))
        .args([
            "--socket",
            dir.join("socket").to_str().unwrap(),
            "text",
            "Hello",
            "World",
        ])
        .output()
        .unwrap();
    assert!(
        input.status.success(),
        "text send failed: {}\n{}",
        String::from_utf8_lossy(&input.stderr).trim_end(),
        wrapper_stderr(&dir)
    );
    let snapshot = request_all(
        &dir,
        r#"{"v":1,"op":"observe"}
"#,
    )
    .into_iter()
    .find(|event| event["event"] == "snapshot")
    .unwrap();
    assert!(snapshot["rows"].to_string().contains("Hello World"));
    let _ = request(
        &dir,
        r#"{"v":1,"op":"shutdown"}
"#,
    );
    child.wait().unwrap();
}

#[test]
fn signal_cleanup_removes_socket() {
    let dir = temp_dir("signal");
    let mut child = start(&dir, &["sleep", "30"], "30");
    for _ in 0..READY_POLLS {
        if dir.join("socket").exists() {
            break;
        }
        thread::sleep(POLL_INTERVAL);
    }
    unsafe {
        libc::kill(child.id() as libc::pid_t, libc::SIGTERM);
    }
    child.wait().unwrap();
    assert!(!dir.join("socket").exists());
}

#[test]
fn idle_timeout_reports_status_124() {
    let dir = temp_dir("idle");
    let mut child = start(&dir, &["sleep", "3"], "1");
    let mut output = String::new();
    child
        .stdout
        .take()
        .unwrap()
        .read_to_string(&mut output)
        .unwrap();
    child.wait().unwrap();
    assert!(
        output
            .lines()
            .any(|line| line.contains("\"reason\":\"idle_timeout\"")
                && line.contains("\"status\":124")),
        "no idle_timeout lifecycle line\n{}",
        wrapper_stderr(&dir)
    );
    assert!(!dir.join("socket").exists());
}

#[test]
fn long_input_and_descendants_are_handled() {
    let dir = temp_dir("long-input");
    let pid_file = dir.join("descendant.pid");
    let command = format!(
        "sleep 30 & echo $! > {}; wc -c >/dev/null; wait",
        pid_file.display()
    );
    let mut child = start(&dir, &["sh", "-c", &command], "5");
    for _ in 0..READY_POLLS {
        if pid_file.exists() {
            break;
        }
        thread::sleep(POLL_INTERVAL);
    }
    let text = "x".repeat(60_000) + "\n";
    let body = serde_json::json!({"v":1,"op":"text","text":text}).to_string() + "\n";
    let ack = request(&dir, &body);
    assert_eq!(ack["event"], "ack");
    let descendant: libc::pid_t = fs::read_to_string(&pid_file)
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    let _ = request(&dir, "{\"v\":1,\"op\":\"shutdown\"}\n");
    child.wait().unwrap();
    for _ in 0..READY_POLLS {
        if unsafe { libc::kill(descendant, 0) } == -1 {
            return;
        }
        thread::sleep(POLL_INTERVAL);
    }
    panic!("descendant survived wrapper cleanup");
}
