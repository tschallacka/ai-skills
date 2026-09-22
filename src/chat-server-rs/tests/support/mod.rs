// MODE: DEV
// Each integration-test file that does `mod support;` compiles its own,
// separate copy of this module as part of its own test binary, so dead-code
// analysis runs per binary: a helper only some OTHER migrated file uses would
// warn (and, under -D warnings, fail the build) here. Suppressed for exactly
// that structural reason, not because any of this is genuinely unused.
#![allow(dead_code)]
//! Shared spawn/readiness/cleanup helpers for the chat/tests migration
//! (T145 goal 25, W131). Every migrated chat/tests file -- in both
//! chat-server-rs and chat-client-rs -- uses this module instead of
//! duplicating the setup/teardown pattern: a scratch AI_CHAT_HOME, a real
//! chat-server-rs subprocess, a readiness poll on `server.port`, and
//! cleanup-on-drop.
//!
//! chat-client-rs includes this file via `#[path = "../../chat-server-rs/tests/support/mod.rs"]
//! mod support;` rather than a new shared crate: neither crate depends on the
//! other in `[dependencies]`, and a real Cargo path dependency between them
//! (even dev-only) would be a heavier answer than a test module genuinely
//! needs.

use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::Duration;

/// How many 100ms polls a readiness wait is allowed: 100, so ten seconds.
/// A CEILING, not a sleep -- a healthy run returns the moment its condition
/// holds.
const READY_POLLS: usize = 100;
const POLL_INTERVAL: Duration = Duration::from_millis(100);

/// Walks up from THIS test binary's own path to the workspace's shared
/// target/{debug,release}[/<triple>] directory -- the directory every
/// sibling package's own binary lands in when built alongside this crate,
/// with no Cargo dependency edge needed to find it.
fn sibling_bin_dir() -> PathBuf {
    let mut dir = std::env::current_exe().expect("current test binary path");
    dir.pop(); // the test binary itself
    if dir.file_name().is_some_and(|name| name == "deps") {
        dir.pop();
    }
    dir
}

/// Resolve `name`'s compiled binary in the shared workspace target directory,
/// building it on demand if it is not already there.
///
/// `cargo test -p chat-server-rs` alone never builds chat-client-rs (there is
/// no Cargo dependency edge between the two crates), so a cross-crate spawn
/// that only LOCATED a path would pass after a full workspace build and fail
/// non-deterministically on an isolated single-package run -- exactly the
/// problem this function exists to solve.
pub fn resolve_workspace_binary(name: &str) -> PathBuf {
    let bin_dir = sibling_bin_dir();
    // A built binary carries the platform's executable suffix (.exe on
    // Windows), which `join(name)` alone leaves off.
    let program = bin_dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    if program.is_file() {
        return program;
    }
    let mut cmd = Command::new(env!("CARGO"));
    cmd.arg("build").arg("-p").arg(name);
    // bin_dir is target/debug (native) or target/<triple>/debug
    // (cross-compiled); an explicit --target is required in the second case
    // or this build would land in target/debug instead, right where bin_dir
    // does NOT point. Walk up from bin_dir past whatever sits above "target"
    // -- one level native, two cross-compiled -- to find the workspace root
    // cargo must run from for its own default output location to match
    // bin_dir.
    if let Some(triple) = bin_dir
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .filter(|name| *name != "target")
    {
        cmd.arg("--target").arg(triple);
    }
    let mut workspace_root = bin_dir.clone();
    loop {
        let popped = workspace_root.file_name().map(|n| n.to_os_string());
        if !workspace_root.pop() {
            panic!("bin_dir has no 'target' ancestor: {}", bin_dir.display());
        }
        if popped.as_deref() == Some(std::ffi::OsStr::new("target")) {
            break;
        }
    }
    let output = cmd
        .current_dir(&workspace_root)
        .output()
        .unwrap_or_else(|error| panic!("could not build {name}: {error}"));
    assert!(
        output.status.success(),
        "building {name} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        program.is_file(),
        "{name} still missing at {} after building it",
        program.display()
    );
    program
}

/// A scratch `AI_CHAT_HOME`-style directory, removed on drop.
pub struct ScratchDir(PathBuf);

impl ScratchDir {
    pub fn new(label: &str) -> Self {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let unique = COUNTER.fetch_add(1, Ordering::Relaxed);
        let mut dir = std::env::temp_dir();
        dir.push(format!("chat-test-{label}-{}-{unique}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        ScratchDir(dir)
    }

    pub fn path(&self) -> &Path {
        &self.0
    }

    pub fn join(&self, part: &str) -> PathBuf {
        self.0.join(part)
    }
}

impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

/// A spawned process, killed (and reaped) on drop so a panicking assertion
/// never leaks a live server or client into the next test.
pub struct ChildGuard(pub Child);

impl ChildGuard {
    pub fn pid(&self) -> u32 {
        self.0.id()
    }

    /// Whether the process is still running. `try_wait` rather than asking
    /// `ps`: it is the same answer on every platform, and Windows has no `ps`
    /// that takes `-o state=`.
    pub fn is_alive(&mut self) -> bool {
        matches!(self.0.try_wait(), Ok(None))
    }
}

impl Drop for ChildGuard {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

/// A running chat-server-rs, its scratch home, and its resolved port.
pub struct ChatServer {
    pub home: ScratchDir,
    pub child: ChildGuard,
    pub port: u16,
}

/// Spawn chat-server-rs against a fresh scratch home, waiting for it to
/// report a port. `extra_env` sets the
/// `CHAT_ANNOUNCE`/`CHAT_BCAST`/`CHAT_BEACON_PORT`/`CHAT_ANNOUNCE_HOST`
/// overrides.
pub fn spawn_server(label: &str, extra_env: &[(&str, &str)]) -> ChatServer {
    let home = ScratchDir::new(label);
    let binary = resolve_workspace_binary("chat-server-rs");
    let mut cmd = Command::new(&binary);
    cmd.arg("0")
        .env("AI_CHAT_HOME", home.path())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for (key, value) in extra_env {
        cmd.env(key, value);
    }
    let child = cmd
        .spawn()
        .unwrap_or_else(|error| panic!("spawning {} failed: {error}", binary.display()));
    let child = ChildGuard(child);

    let port_file = home.join("server.port");
    let mut port = None;
    for _ in 0..READY_POLLS {
        if let Ok(text) = fs::read_to_string(&port_file) {
            if let Ok(parsed) = text.trim().parse::<u16>() {
                port = Some(parsed);
                break;
            }
        }
        thread::sleep(POLL_INTERVAL);
    }
    let port = port.unwrap_or_else(|| {
        panic!(
            "chat-server-rs never reported a port at {}",
            port_file.display()
        )
    });
    ChatServer { home, child, port }
}

/// Run chat-client-rs once, with its own scratch `AI_CHAT_HOME`, and return
/// its captured stdout as a `String`. One client state dir per operation,
/// since the TOFU cert pin and the session cursor are both per-directory.
pub fn run_client(home: &Path, args: &[&str]) -> std::process::Output {
    let binary = resolve_workspace_binary("chat-client-rs");
    Command::new(&binary)
        .args(args)
        .env("AI_CHAT_HOME", home)
        .output()
        .unwrap_or_else(|error| panic!("running {} failed: {error}", binary.display()))
}

/// A UDP port nothing holds right now, as a string ready for an env var or an
/// argv slot. Discovery tests need a beacon port of their own: a literal shared
/// by two overlapping test runs (the pre-push gate beside a suite run, two
/// worktrees) makes each run hear the other's beacon or fail to bind, and the
/// loser reports a bare `cargo test` failure that vanishes when re-run alone.
/// Bound on the wildcard address because that is what the client binds.
pub fn free_udp_port() -> String {
    let probe =
        std::net::UdpSocket::bind((std::net::Ipv4Addr::UNSPECIFIED, 0)).expect("a free UDP port");
    probe
        .local_addr()
        .expect("bound address")
        .port()
        .to_string()
}

pub fn stdout_string(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

pub fn stderr_string(output: &std::process::Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// Read a JSON record file (e.g. an owner-socket record), returning `Value::Null`
/// if it does not exist yet.
pub fn read_json(path: &Path) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or(Value::Null)
}
