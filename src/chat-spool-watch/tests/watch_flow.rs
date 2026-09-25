// MODE: DEV
//! The real `chat-spool-watch` binary against a real spool directory: what it
//! prints and when, that it never empties the spool, and that its heartbeat
//! exists while it runs and is gone after a clean exit.

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, SystemTime};

fn scratch(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "chat-spool-watch-flow-{tag}-{}-{:?}",
        std::process::id(),
        SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn spool(state: &Path, session: &str) -> PathBuf {
    let dir = state.join("interrupts").join(session);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn watcher(state: &Path, session: &str) -> Command {
    let mut command = Command::new(env!("CARGO_BIN_EXE_chat-spool-watch"));
    command
        .args([
            "--state",
            &state.display().to_string(),
            "--session",
            session,
        ])
        .env_remove("CLAUDE_CODE_SESSION_ID")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    command
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn unread_notices_are_reported_once_and_left_in_the_spool() {
    let state = scratch("stale");
    let dir = spool(&state, "sess-1");
    std::fs::write(
        dir.join("me.log"),
        "[10:00:00Z] #ops <alice> the deploy failed\n",
    )
    .unwrap();

    let output = watcher(&state, "sess-1")
        .args(["--after", "0.3", "--poll", "0.1", "--once"])
        .output()
        .unwrap();

    assert_eq!(output.status.code(), Some(0));
    let out = stdout(&output);
    assert_eq!(out.lines().count(), 1, "{out}");
    assert!(out.contains("1 interrupt unread"), "{out}");
    assert!(out.contains("alice> the deploy failed"), "{out}");
    assert!(
        dir.join("me.log").is_file(),
        "the watcher must never empty the spool"
    );
    let _ = std::fs::remove_dir_all(&state);
}

#[test]
fn notices_read_in_time_are_never_reported() {
    let state = scratch("drained");
    let dir = spool(&state, "sess-2");
    std::fs::write(dir.join("me.log"), "[10:00:00Z] a\n").unwrap();
    let file = dir.join("me.log");
    // Stand in for the hook: take the notice well inside the time allowed.
    let taker = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(300));
        std::fs::remove_file(file).unwrap();
    });

    let output = watcher(&state, "sess-2")
        .args(["--after", "1.5", "--poll", "0.1", "--max-runtime", "2.5"])
        .output()
        .unwrap();
    taker.join().unwrap();

    assert_eq!(output.status.code(), Some(0));
    assert_eq!(
        stdout(&output),
        "",
        "it spoke about a spool that was emptied"
    );
    let _ = std::fs::remove_dir_all(&state);
}

#[test]
fn the_heartbeat_exists_while_it_runs_and_is_gone_after_a_clean_exit() {
    let state = scratch("heartbeat");
    let mut child = watcher(&state, "sess-3")
        .args(["--after", "60", "--poll", "0.1", "--max-runtime", "1.5"])
        .spawn()
        .unwrap();
    let beat = state.join("interrupts/sess-3").join(".watcher");
    let deadline = std::time::Instant::now() + Duration::from_secs(5);
    while !beat.is_file() && std::time::Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(beat.is_file(), "no heartbeat while the watcher runs");
    let status = child.wait().unwrap();
    assert_eq!(status.code(), Some(0));
    assert!(!beat.exists(), "the heartbeat was left behind");
    let _ = std::fs::remove_dir_all(&state);
}

#[test]
fn another_sessions_spool_is_not_watched() {
    let state = scratch("other");
    let other = spool(&state, "someone-else");
    std::fs::write(other.join("x.log"), "[10:00:00Z] not mine\n").unwrap();
    let output = watcher(&state, "sess-4")
        .args(["--after", "0.2", "--poll", "0.1", "--max-runtime", "1"])
        .output()
        .unwrap();
    assert_eq!(stdout(&output), "");
    let _ = std::fs::remove_dir_all(&state);
}

#[test]
fn no_session_id_is_a_usage_error() {
    let output = Command::new(env!("CARGO_BIN_EXE_chat-spool-watch"))
        .env_remove("CLAUDE_CODE_SESSION_ID")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(64));
    assert!(String::from_utf8_lossy(&output.stderr).contains("no session id"));
}

#[test]
fn a_bad_number_is_refused_by_name_and_help_prints_the_usage() {
    let output = Command::new(env!("CARGO_BIN_EXE_chat-spool-watch"))
        .args(["--session", "s", "--after", "soon"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(64));
    assert!(String::from_utf8_lossy(&output.stderr).contains("--after needs a number"));

    let help = Command::new(env!("CARGO_BIN_EXE_chat-spool-watch"))
        .arg("--help")
        .output()
        .unwrap();
    assert_eq!(help.status.code(), Some(0));
    assert!(stdout(&help).contains("Monitor"));
}
