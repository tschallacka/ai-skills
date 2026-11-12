// MODE: DEV
//! Real subprocess-level integration tests: the real compiled
//! tailpipe-server-rs binary, started as a genuine child process with its
//! own socket, driven by this crate's own transport::Stream -- not the
//! crate's in-process hub::Hub unit tests.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::time::Duration;
use tailpipe_server_rs::protocol::{Request, Response};
use tailpipe_server_rs::transport::Stream;

struct Server {
    child: Child,
    endpoint: PathBuf,
    dir: PathBuf,
}

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

impl Server {
    fn start(extra_args: &[&str]) -> Self {
        let dir = short_temp_dir().join(format!(
            "tailpipe-server-flow-{}-{}",
            std::process::id(),
            unique()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let endpoint = dir.join("tailpipe.sock");
        let snapshot_dir = dir.join("snapshots");

        let mut args: Vec<String> = vec![
            endpoint.display().to_string(),
            "--snapshot-dir".to_string(),
            snapshot_dir.display().to_string(),
        ];
        args.extend(extra_args.iter().map(|arg| arg.to_string()));

        let child = Command::new(env!("CARGO_BIN_EXE_tailpipe-server-rs"))
            .args(&args)
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
                // Give the listener a moment past the file appearing (the
                // Windows discovery-file arm writes the file before the
                // socket is necessarily ready for a connect race-free).
                std::thread::sleep(Duration::from_millis(20));
                return;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        panic!("server endpoint never appeared: {:?}", self.endpoint);
    }

    fn snapshot_dir(&self) -> PathBuf {
        self.dir.join("snapshots")
    }

    fn request(&self, request: &Request) -> Response {
        let mut stream = connect_with_retry(&self.endpoint);
        writeln!(stream, "{}", serde_json::to_string(request).unwrap()).unwrap();
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        serde_json::from_str(line.trim_end()).unwrap()
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

fn connect_with_retry(endpoint: &Path) -> Stream {
    let mut last_error = None;
    for _ in 0..50 {
        match Stream::connect(endpoint) {
            Ok(stream) => return stream,
            Err(error) => {
                last_error = Some(error);
                std::thread::sleep(Duration::from_millis(20));
            }
        }
    }
    panic!("could not connect to server: {last_error:?}");
}

fn unique() -> u64 {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

#[test]
fn a_line_piped_in_is_retrievable_by_id_and_range() {
    let server = Server::start(&[]);

    let id = match server.request(&Request::Ingest {
        stream: "s".into(),
        line: "hello world".into(),
    }) {
        Response::Ingested { id } => id,
        other => panic!("unexpected response: {other:?}"),
    };
    assert_eq!(id, 1);

    match server.request(&Request::Read {
        stream: "s".into(),
        from: id,
        to: id,
    }) {
        Response::Lines { lines } => {
            assert_eq!(lines.len(), 1);
            assert_eq!(lines[0].text, "hello world");
        }
        other => panic!("unexpected response: {other:?}"),
    }
}

#[test]
fn an_idle_stream_is_evicted_and_its_gzip_snapshot_is_readable() {
    let server = Server::start(&["--idle-timeout-ms", "200"]);

    server.request(&Request::Ingest {
        stream: "idle".into(),
        line: "soon to be evicted".into(),
    });

    // The sweep itself runs every 30s in production; for this test the
    // important thing is the idle threshold, so poll (comfortably past one
    // full sweep interval) until the stream is gone rather than hard-coding
    // the sweep interval here too.
    let mut names = vec!["idle".to_string()];
    for _ in 0..400 {
        match server.request(&Request::List) {
            Response::Streams { names: current } => {
                names = current;
                if !names.contains(&"idle".to_string()) {
                    break;
                }
            }
            other => panic!("unexpected response: {other:?}"),
        }
        std::thread::sleep(Duration::from_millis(100));
    }
    assert!(
        !names.contains(&"idle".to_string()),
        "stream was never evicted"
    );

    let snapshot = std::fs::read_dir(server.snapshot_dir())
        .unwrap()
        .find_map(|entry| entry.ok())
        .expect("a snapshot file was written");
    let mut decoder = flate2::read::GzDecoder::new(std::fs::File::open(snapshot.path()).unwrap());
    let mut text = String::new();
    std::io::Read::read_to_string(&mut decoder, &mut text).unwrap();
    assert_eq!(text, "soon to be evicted\n");
}

/// The server process's own live thread count, or None where this platform
/// has no cheap way to ask. One thread per connection is the server's whole
/// concurrency model, so this is the count GH #197 watched grow to 434.
#[cfg(target_os = "linux")]
fn thread_count(pid: u32) -> Option<usize> {
    std::fs::read_to_string(format!("/proc/{pid}/status"))
        .ok()?
        .lines()
        .find_map(|line| line.strip_prefix("Threads:"))
        .and_then(|count| count.trim().parse().ok())
}

/// `ps -M` prints a header and then one row per thread.
#[cfg(target_os = "macos")]
fn thread_count(pid: u32) -> Option<usize> {
    let output = Command::new("ps")
        .args(["-M", "-p", &pid.to_string()])
        .output()
        .ok()?;
    let rows = String::from_utf8_lossy(&output.stdout).lines().count();
    rows.checked_sub(1)
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn thread_count(_pid: u32) -> Option<usize> {
    None
}

/// Polls until `done` holds for the server's thread count, returning the
/// last count seen. A ceiling, not a sleep: the healthy case returns at once.
fn wait_for_threads(pid: u32, budget: Duration, done: impl Fn(usize) -> bool) -> usize {
    let deadline = std::time::Instant::now() + budget;
    loop {
        let count = thread_count(pid).expect("thread count is readable");
        if done(count) || std::time::Instant::now() >= deadline {
            return count;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Opens a Tail connection and leaves it for the caller to drop.
fn open_tail(server: &Server, stream: &str) -> Stream {
    let mut tail = connect_with_retry(&server.endpoint);
    let request = Request::Tail {
        stream: stream.into(),
        since: 0,
    };
    writeln!(tail, "{}", serde_json::to_string(&request).unwrap()).unwrap();
    tail
}

/// GH #197: a Tail whose client has gone must end its thread. Before the fix
/// a tail's thread had no stop condition at all and ignored its write
/// errors, so every abandoned tail (a killed `tail`, an MCP `wait` that timed
/// out) kept a thread waking 50 times a second for the life of the server.
#[test]
fn a_tail_whose_client_has_gone_releases_its_thread() {
    let server = Server::start(&[]);
    let pid = server.child.id();
    if thread_count(pid).is_none() {
        eprintln!("SKIP: no thread count on this platform");
        return;
    }
    server.request(&Request::Ingest {
        stream: "busy".into(),
        line: "first".into(),
    });
    let baseline = wait_for_threads(pid, Duration::from_secs(5), |_| true);

    // Half tail a stream that keeps receiving lines (a gone client's writes
    // fail), half a stream that never exists (nothing is ever written, so
    // only noticing the closed socket can end them).
    const TAILS: usize = 40;
    let tails: Vec<Stream> = (0..TAILS)
        .map(|n| open_tail(&server, if n % 2 == 0 { "busy" } else { "silent" }))
        .collect();
    let peak = wait_for_threads(pid, Duration::from_secs(10), |count| {
        count >= baseline + TAILS
    });
    assert!(
        peak >= baseline + TAILS,
        "the tails never all started: baseline {baseline}, peak {peak}"
    );

    drop(tails);
    for n in 0..20 {
        server.request(&Request::Ingest {
            stream: "busy".into(),
            line: format!("after the clients left {n}"),
        });
    }

    let settled = wait_for_threads(pid, Duration::from_secs(10), |count| count <= baseline);
    assert!(
        settled <= baseline,
        "abandoned tails kept their threads: baseline {baseline}, peak {peak}, \
         still {settled} after the clients left"
    );
}

/// The other half of GH #197's fix: noticing a gone client must not end a
/// tail whose client is still there but has simply had nothing to read for
/// longer than the liveness check's own interval.
#[test]
fn a_live_tail_survives_a_quiet_spell_and_still_receives_lines() {
    let server = Server::start(&[]);
    let tail = open_tail(&server, "quiet");
    tail.set_read_timeout(Some(Duration::from_secs(10)))
        .unwrap();
    std::thread::sleep(Duration::from_millis(2500));
    server.request(&Request::Ingest {
        stream: "quiet".into(),
        line: "after the quiet spell".into(),
    });

    let mut reader = BufReader::new(tail);
    let mut line = String::new();
    reader.read_line(&mut line).unwrap();
    match serde_json::from_str(line.trim_end()).unwrap() {
        Response::Lines { lines } => assert_eq!(lines[0].text, "after the quiet spell"),
        other => panic!("unexpected response: {other:?}"),
    }
}
