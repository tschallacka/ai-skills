// MODE: DEV
//! End-to-end checks of the lock against the real binary: a signalled monitor stops its
//! watch, a killed monitor leaves its watch holding the lock, and simultaneous starts
//! elect exactly one monitor. Unix only, since they send signals and inspect process groups.

#![cfg(unix)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

const BIN: &str = env!("CARGO_BIN_EXE_monitor-once");

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("monitor-once-it-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

/// The watched command's process group, read from the lock once it is recorded.
fn wait_for_group(lock: &Path) -> i32 {
    // A ceiling counted in polls, not a wall-clock deadline: a paused CI runner resumes the count
    // instead of finding it already spent (.agents/knowledge/github-ci-runners.md).
    for _ in 0..100 {
        if let Ok(text) = fs::read_to_string(lock) {
            if let Some(group) = text
                .lines()
                .nth(2)
                .and_then(|s| s.trim().parse::<i32>().ok())
            {
                if group > 0 {
                    return group;
                }
            }
        }
        thread::sleep(Duration::from_millis(50));
    }
    panic!("the lock never recorded the watch's group");
}

fn group_alive(group: i32) -> bool {
    unsafe { libc::kill(-group, 0) == 0 }
}

fn signal(pid: i32, sig: i32) {
    unsafe {
        libc::kill(pid, sig);
    }
}

fn start(dir: &Path, name: &str, command: &[&str]) -> std::process::Child {
    Command::new(BIN)
        .args(["--name", name, "--dir"])
        .arg(dir)
        .arg("--")
        .args(command)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap()
}

#[test]
fn sigterm_to_the_monitor_stops_the_watch_and_frees_the_lock() {
    let dir = scratch("sigterm");
    let mut monitor = start(&dir, "watch", &["sleep", "60"]);
    let group = wait_for_group(&dir.join("watch.pid"));
    signal(monitor.id() as i32, libc::SIGTERM);
    let status = monitor.wait().unwrap();
    assert_eq!(
        status.code(),
        Some(143),
        "a monitor stopped by SIGTERM exits 128+15"
    );
    thread::sleep(Duration::from_millis(300));
    assert!(
        !group_alive(group),
        "the watch must not outlive its monitor"
    );
    assert!(!dir.join("watch.pid").exists(), "the lock must be released");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn a_killed_monitor_leaves_the_lock_held_while_its_watch_lives() {
    let dir = scratch("sigkill");
    let mut monitor = start(&dir, "watch", &["sleep", "60"]);
    let group = wait_for_group(&dir.join("watch.pid"));
    signal(monitor.id() as i32, libc::SIGKILL);
    monitor.wait().unwrap();
    thread::sleep(Duration::from_millis(200));
    assert!(
        group_alive(group),
        "the watch survives a SIGKILL of its monitor"
    );
    let refused = start(&dir, "watch", &["true"]).wait().unwrap();
    assert_eq!(
        refused.code(),
        Some(75),
        "a second start is refused while the watch lives"
    );
    signal(-group, libc::SIGTERM);
    // Retried, not a fixed 300 ms: the group's exit is not instant on a
    // loaded host, and a start while it lives is refused with 75.
    let mut allowed = None;
    for _ in 0..100 {
        allowed = start(&dir, "watch", &["true"]).wait().unwrap().code();
        if allowed != Some(75) {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    assert_eq!(allowed, Some(0), "once the watch is gone the lock is free");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn different_nicks_run_the_same_watch_and_one_nick_runs_it_once() {
    let dir = scratch("nicks");
    let start_as = |nick: &str| {
        Command::new(BIN)
            .args(["--name", "watch", "--nick", nick, "--dir"])
            .arg(&dir)
            .args(["--", "sleep", "5"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .unwrap()
    };
    let mut alice = start_as("agent-alice");
    let mut bob = start_as("agent-bob");
    // Polled, not a fixed 200 ms: under a loaded gate the second spawn had
    // not written its pid file yet. The watch sleeps 5 s, so it is still
    // running when the same-nick refusal below is checked.
    let both =
        || dir.join("watch.agent-alice.pid").exists() && dir.join("watch.agent-bob.pid").exists();
    for _ in 0..100 {
        if both() {
            break;
        }
        thread::sleep(Duration::from_millis(50));
    }
    assert!(dir.join("watch.agent-alice.pid").exists());
    assert!(dir.join("watch.agent-bob.pid").exists());
    let again = start_as("agent-alice").wait().unwrap();
    assert_eq!(
        again.code(),
        Some(75),
        "the same nick is refused while its watch runs"
    );
    assert_eq!(alice.wait().unwrap().code(), Some(0));
    assert_eq!(bob.wait().unwrap().code(), Some(0));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn simultaneous_starts_elect_exactly_one_monitor() {
    let dir = scratch("race");
    let mut children: Vec<_> = (0..8)
        .map(|_| start(&dir, "race", &["sleep", "1"]))
        .collect();
    let codes: Vec<Option<i32>> = children
        .iter_mut()
        .map(|child| child.wait().unwrap().code())
        .collect();
    let winners = codes.iter().filter(|code| **code == Some(0)).count();
    let refused = codes.iter().filter(|code| **code == Some(75)).count();
    assert_eq!(winners, 1, "exactly one start runs the watch: {codes:?}");
    assert_eq!(refused, 7, "every other start is refused: {codes:?}");
    let _ = fs::remove_dir_all(&dir);
}
