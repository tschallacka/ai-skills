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
    Command::new(env!("CARGO_BIN_EXE_interactive-shell"))
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
    let _ = request(
        &dir,
        r#"{"v":1,"op":"shutdown"}
"#,
    );
    child.wait().unwrap();
    assert!(!dir.join("socket").exists());
}
