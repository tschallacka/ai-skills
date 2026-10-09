// MODE: DEV
//! Real subprocess-level integration tests: the real compiled
//! tailpipe-server-rs binary driven by the real compiled tailpipe-client-rs
//! binary (std::process::Command, not this crate's own library functions,
//! which client.rs's/ingest.rs's own unit tests already cover).

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;

struct Server {
    child: Child,
    endpoint: PathBuf,
    dir: PathBuf,
}

impl Server {
    fn start() -> Self {
        let dir = std::env::temp_dir().join(format!(
            "tailpipe-cli-flow-{}-{}",
            std::process::id(),
            unique()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let endpoint = dir.join("tailpipe.sock");
        let snapshot_dir = dir.join("snapshots");
        let child = Command::new(sibling_bin_path("tailpipe-server-rs"))
            .arg(&endpoint)
            .arg("--snapshot-dir")
            .arg(&snapshot_dir)
            .spawn()
            .expect("start tailpipe-server-rs");
        let server = Server {
            child,
            endpoint,
            dir,
        };
        server.wait_for_endpoint();
        server
    }

    fn wait_for_endpoint(&self) {
        for _ in 0..100 {
            if self.endpoint.exists() {
                std::thread::sleep(Duration::from_millis(20));
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("server endpoint never appeared: {:?}", self.endpoint);
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn sibling_bin_path(name: &str) -> PathBuf {
    let mut dir = std::env::current_exe().expect("current test binary path");
    dir.pop();
    if dir.file_name().is_some_and(|part| part == "deps") {
        dir.pop();
    }
    dir.join(name)
}

fn unique() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

fn run_client(args: &[&str], stdin_text: Option<&str>) -> String {
    let mut command = Command::new(env!("CARGO_BIN_EXE_tailpipe-client-rs"));
    command.args(args);
    if stdin_text.is_some() {
        command.stdin(Stdio::piped());
    }
    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    let mut child = command.spawn().expect("start tailpipe-client-rs");
    if let Some(text) = stdin_text {
        child
            .stdin
            .take()
            .unwrap()
            .write_all(text.as_bytes())
            .unwrap();
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "tailpipe-client-rs {args:?} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn ingest_list_read_and_save_round_trip() {
    let server = Server::start();
    let endpoint = server.endpoint.to_str().unwrap();

    run_client(
        &["ingest", "--endpoint", endpoint, "--stream", "s"],
        Some("alpha\nbeta\ngamma\n"),
    );

    let list_out = run_client(&["list", "--endpoint", endpoint], None);
    assert!(list_out.lines().any(|line| line == "s"));

    let read_out = run_client(
        &[
            "read",
            "--endpoint",
            endpoint,
            "--stream",
            "s",
            "--from",
            "1",
            "--to",
            "3",
        ],
        None,
    );
    let lines: Vec<&str> = read_out.lines().collect();
    assert_eq!(lines.len(), 3);
    assert_eq!(lines[0], "1\talpha");
    assert_eq!(lines[1], "2\tbeta");
    assert_eq!(lines[2], "3\tgamma");

    let save_out = run_client(&["save", "--endpoint", endpoint, "--stream", "s"], None);
    let path = Path::new(save_out.trim());
    assert!(
        path.exists(),
        "save did not report an existing path: {path:?}"
    );

    let gunzip_status = Command::new("gunzip")
        .arg("-t")
        .arg(path)
        .status()
        .expect("run gunzip -t");
    assert!(
        gunzip_status.success(),
        "gunzip -t reported the snapshot as corrupt"
    );
}

#[test]
fn search_finds_a_line_by_exact_text_and_by_regex() {
    let server = Server::start();
    let endpoint = server.endpoint.to_str().unwrap();

    run_client(
        &["ingest", "--endpoint", endpoint, "--stream", "s"],
        Some("error: disk full\nall good\nerror: out of memory\n"),
    );

    let exact_out = run_client(
        &[
            "search",
            "--endpoint",
            endpoint,
            "--stream",
            "s",
            "--mode",
            "exact",
            "--query",
            "disk full",
        ],
        None,
    );
    let exact_lines: Vec<&str> = exact_out.lines().collect();
    assert_eq!(exact_lines.len(), 1);
    assert_eq!(exact_lines[0], "1\terror: disk full");

    let regex_out = run_client(
        &[
            "search",
            "--endpoint",
            endpoint,
            "--stream",
            "s",
            "--mode",
            "regex",
            "--query",
            "^error:",
        ],
        None,
    );
    let regex_lines: Vec<&str> = regex_out.lines().collect();
    assert_eq!(regex_lines.len(), 2);
    assert_eq!(regex_lines[0], "1\terror: disk full");
    assert_eq!(regex_lines[1], "3\terror: out of memory");
}

#[test]
fn tail_follows_new_lines_as_they_are_ingested() {
    let server = Server::start();
    let endpoint = server.endpoint.to_str().unwrap().to_string();

    let mut tail_child = Command::new(env!("CARGO_BIN_EXE_tailpipe-client-rs"))
        .args(["tail", "--endpoint", &endpoint, "--stream", "s"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("start tailpipe-client-rs tail");
    let tail_stdout = tail_child.stdout.take().unwrap();
    let (sender, receiver) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        for line in BufReader::new(tail_stdout).lines().map_while(Result::ok) {
            let _ = sender.send(line);
        }
    });

    // Give the tail subcommand a moment to connect and resolve "current
    // end" before the second process ingests, so this proves live
    // delivery rather than a startup-order fluke.
    std::thread::sleep(Duration::from_millis(200));
    run_client(
        &["ingest", "--endpoint", &endpoint, "--stream", "s"],
        Some("pushed while tailing\n"),
    );

    let received = receiver
        .recv_timeout(Duration::from_secs(5))
        .expect("tail never delivered the ingested line");
    assert_eq!(received, "1\tpushed while tailing");

    let _ = tail_child.kill();
    let _ = tail_child.wait();
}
