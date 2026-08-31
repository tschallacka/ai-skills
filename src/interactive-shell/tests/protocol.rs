use serde_json::Value;
use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::UnixStream;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

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
        .stderr(Stdio::null())
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
    for _ in 0..100 {
        if let Ok(mut stream) = UnixStream::connect(dir.join("socket")) {
            stream.write_all(body.as_bytes()).unwrap();
            stream.shutdown(std::net::Shutdown::Write).unwrap();
            let mut out = String::new();
            stream.read_to_string(&mut out).unwrap();
            return serde_json::from_str(out.trim()).unwrap();
        }
        thread::sleep(Duration::from_millis(10));
    }
    panic!("socket did not appear")
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
    assert!(!screens.is_empty());
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
    assert!(rows.contains("CSI_SAFE"));
    assert!(rows.contains("OSC_SAFE"));
    assert!(!rows.contains(&"1".repeat(129)));
    assert!(!rows.contains(&"x".repeat(4097)));
}

#[test]
fn socket_is_private_and_invalid_requests_are_rejected() {
    let dir = temp_dir("bounds");
    let mut child = start(&dir, &["sleep", "2"], "5");
    for _ in 0..100 {
        if dir.join("socket").exists() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
    }
    let mode = fs::metadata(dir.join("socket"))
        .unwrap()
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
            "text",
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
    assert!(upper.status.success());
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
fn malformed_cli_arguments_do_not_panic() {
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
fn signal_cleanup_removes_socket() {
    let dir = temp_dir("signal");
    let mut child = start(&dir, &["sleep", "30"], "30");
    for _ in 0..100 {
        if dir.join("socket").exists() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
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
    assert!(output.lines().any(
        |line| line.contains("\"reason\":\"idle_timeout\"") && line.contains("\"status\":124")
    ));
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
    for _ in 0..1000 {
        if pid_file.exists() {
            break;
        }
        thread::sleep(Duration::from_millis(10));
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
    for _ in 0..100 {
        if unsafe { libc::kill(descendant, 0) } == -1 {
            return;
        }
        thread::sleep(Duration::from_millis(10));
    }
    panic!("descendant survived wrapper cleanup");
}
