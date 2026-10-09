// MODE: DEV
//! Real subprocess-level integration test: the real compiled
//! tailpipe-server-rs binary, ingested into via the real compiled
//! tailpipe-client-rs binary, and queried through the real compiled
//! tailpipe-mcp binary over genuine stdio JSON-RPC -- not this crate's own
//! in-process `handle` unit tests.

use serde_json::{json, Value};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

/// A unix domain socket's sun_path has a small platform-defined limit (104
/// bytes on macOS, 108 on Linux). macOS's own $TMPDIR is already ~49 bytes
/// before anything of this test's own naming is added (github-ci-
/// runners.md "$TMPDIR is long enough to break Unix sockets"), so plain
/// /tmp is used directly on unix instead -- matching
/// planning-server/src/endpoint.rs's own short_root, and
/// verify-both-shells.sh's own per-test /tmp/t.XXXXX.
fn short_temp_dir() -> PathBuf {
    #[cfg(unix)]
    {
        PathBuf::from("/tmp")
    }
    #[cfg(not(unix))]
    {
        std::env::temp_dir()
    }
}

/// Workspace sibling binaries all land in the same target/{debug,release}
/// directory this test binary itself was built into.
fn sibling_bin_dir() -> PathBuf {
    Path::new(env!("CARGO_BIN_EXE_tailpipe-mcp"))
        .parent()
        .expect("the adapter binary lives in a directory")
        .to_path_buf()
}

/// Builds `name` into `bin_dir` if it is not there yet -- see
/// src/tailpipe-client-rs/src/client.rs's own `ensure_built` (mirrored
/// here, same reasoning: tailpipe-server-rs/tailpipe-client-rs are library
/// dependencies of this crate, not bin/artifact ones, so cargo gives no
/// guarantee their [[bin]] targets exist yet when this crate's own tests
/// run in isolation -- observed for real in CI, "No such file or
/// directory" starting tailpipe-server-rs, but never locally where a prior
/// full build had already staged it).
fn ensure_built(bin_dir: &Path, name: &str) -> PathBuf {
    let program = bin_dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
    if program.is_file() {
        return program;
    }
    let mut cmd = Command::new(env!("CARGO"));
    cmd.arg("build").arg("-p").arg(name);
    if let Some(triple) = bin_dir
        .parent()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .filter(|name| *name != "target")
    {
        cmd.arg("--target").arg(triple);
    }
    let mut workspace_root = bin_dir.to_path_buf();
    loop {
        let popped = workspace_root.file_name().map(|n| n.to_os_string());
        if !workspace_root.pop() {
            panic!("bin_dir has no 'target' ancestor: {}", bin_dir.display());
        }
        if popped.as_deref() == Some(std::ffi::OsStr::new("target")) {
            break;
        }
    }
    static BUILDING: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _one_at_a_time = BUILDING.lock().unwrap_or_else(|p| p.into_inner());
    if program.is_file() {
        return program;
    }
    let output = cmd
        .arg("--message-format=json-render-diagnostics")
        .current_dir(&workspace_root)
        .output()
        .unwrap_or_else(|error| panic!("could not build {name}: {error}"));
    assert!(
        output.status.success(),
        "building {name} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    if !program.is_file() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let built = stdout
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line).ok())
            .filter(|message| {
                message["reason"] == "compiler-artifact" && message["target"]["name"] == name
            })
            .filter_map(|message| message["executable"].as_str().map(PathBuf::from))
            .next_back();
        if let Some(built) = built.filter(|path| path.is_file()) {
            std::fs::copy(&built, &program).unwrap_or_else(|error| {
                panic!("copy {} to {}: {error}", built.display(), program.display())
            });
        }
    }
    assert!(
        program.is_file(),
        "{name} still missing at {} after building it; cargo reported:\n{}\n{}",
        program.display(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    program
}

fn scratch() -> PathBuf {
    let dir = short_temp_dir().join(format!(
        "tailpipe-mcp-flow-{}-{}",
        std::process::id(),
        unique()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn unique() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

struct Harness {
    server: Child,
    endpoint: PathBuf,
    adapter: Child,
    stdin: ChildStdin,
    lines: Receiver<String>,
    dir: PathBuf,
    next_id: u64,
}

impl Harness {
    fn start() -> Self {
        let dir = scratch();
        let endpoint = dir.join("tailpipe.sock");
        let snapshot_dir = dir.join("snapshots");

        let server = Command::new(ensure_built(&sibling_bin_dir(), "tailpipe-server-rs"))
            .arg(&endpoint)
            .arg("--snapshot-dir")
            .arg(&snapshot_dir)
            .spawn()
            .expect("start tailpipe-server-rs");
        for _ in 0..100 {
            if endpoint.exists() {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        std::thread::sleep(Duration::from_millis(20));

        let mut adapter = Command::new(env!("CARGO_BIN_EXE_tailpipe-mcp"))
            .env("TAILPIPE_ENDPOINT", &endpoint)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .spawn()
            .expect("start tailpipe-mcp");
        let stdin = adapter.stdin.take().unwrap();
        let stdout = adapter.stdout.take().unwrap();
        let (sender, lines) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines().map_while(Result::ok) {
                let _ = sender.send(line);
            }
        });

        Harness {
            server,
            endpoint,
            adapter,
            stdin,
            lines,
            dir,
            next_id: 1,
        }
    }

    fn call(&mut self, name: &str, arguments: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        let message = json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":name,"arguments":arguments}});
        writeln!(self.stdin, "{message}").unwrap();
        loop {
            let line = self
                .lines
                .recv_timeout(Duration::from_secs(5))
                .expect("adapter never answered");
            let response: Value = serde_json::from_str(&line).unwrap();
            if response["id"] == json!(id) {
                return response;
            }
        }
    }

    fn tool_result(response: &Value) -> Value {
        let text = response["result"]["content"][0]["text"].as_str().unwrap();
        serde_json::from_str(text).unwrap()
    }

    /// Ingests via the real compiled client binary, matching how W16's own
    /// integration test drives it.
    fn ingest(&self, stream: &str, line: &str) {
        let mut child = Command::new(ensure_built(&sibling_bin_dir(), "tailpipe-client-rs"))
            .args([
                "ingest",
                "--endpoint",
                self.endpoint.to_str().unwrap(),
                "--stream",
                stream,
            ])
            .stdin(Stdio::piped())
            .spawn()
            .expect("start tailpipe-client-rs ingest");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(format!("{line}\n").as_bytes())
            .unwrap();
        let status = child.wait().unwrap();
        assert!(status.success(), "ingest failed");
    }
}

impl Drop for Harness {
    fn drop(&mut self) {
        let _ = self.adapter.kill();
        let _ = self.adapter.wait();
        let _ = self.server.kill();
        let _ = self.server.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn all_five_tools_round_trip_through_the_adapter() {
    let mut harness = Harness::start();
    harness.ingest("s", "hello world");

    let list_response = harness.call("list_streams", json!({}));
    let list_result = Harness::tool_result(&list_response);
    assert_eq!(list_result["streams"], json!(["s"]));

    let read_response = harness.call("read", json!({"stream": "s", "from": 1, "to": 1}));
    let read_result = Harness::tool_result(&read_response);
    assert_eq!(
        read_result["lines"],
        json!([{"id": 1, "text": "hello world"}])
    );

    let search_response = harness.call(
        "search",
        json!({"stream": "s", "mode": "exact", "query": "hello"}),
    );
    let search_result = Harness::tool_result(&search_response);
    assert_eq!(
        search_result["lines"],
        json!([{"id": 1, "text": "hello world"}])
    );

    let save_response = harness.call("save", json!({"stream": "s"}));
    let save_result = Harness::tool_result(&save_response);
    let saved_path = save_result["path"].as_str().unwrap();
    assert!(
        Path::new(saved_path).exists(),
        "save did not report an existing path"
    );

    // wait's own non-blocking-concurrency case: send the wait call, then
    // ingest a second line from a concurrent process while it is pending,
    // and confirm it returns promptly with that line rather than timing out.
    let id = harness.next_id;
    harness.next_id += 1;
    let wait_message = json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":{"name":"wait","arguments":{"stream":"s","since":1,"timeout_seconds":10}}});
    writeln!(harness.stdin, "{wait_message}").unwrap();

    std::thread::sleep(Duration::from_millis(200));
    harness.ingest("s", "pushed while waiting");

    let wait_response = loop {
        let line = harness
            .lines
            .recv_timeout(Duration::from_secs(5))
            .expect("adapter never answered wait");
        let response: Value = serde_json::from_str(&line).unwrap();
        if response["id"] == json!(id) {
            break response;
        }
    };
    let wait_result = Harness::tool_result(&wait_response);
    assert_eq!(
        wait_result["line"],
        json!({"id": 2, "text": "pushed while waiting"})
    );
}
