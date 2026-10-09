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

impl Server {
    fn start(extra_args: &[&str]) -> Self {
        let dir = std::env::temp_dir().join(format!(
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
