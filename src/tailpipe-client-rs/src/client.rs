// MODE: DEV
// PACKAGE: PROD
//! The wire-level client: `connect_and_request` for the five single-round-
//! trip verbs, and a separate `tail` for the long-poll Tail request (see
//! tailpipe_server_rs::protocol's own doc comment for why it cannot share
//! connect_and_request's shape).

use std::io::{BufRead, BufReader, Write};
use std::path::Path;
use tailpipe_server_rs::protocol::{Request, Response};
use tailpipe_server_rs::transport::Stream;

/// Opens a fresh connection, sends `request`, and reads back exactly one
/// Response line. Used for every verb except Tail.
pub fn connect_and_request(endpoint: &Path, request: &Request) -> std::io::Result<Response> {
    let mut stream = Stream::connect(endpoint)?;
    writeln!(stream, "{}", serde_json::to_string(request).unwrap())?;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    reader.read_line(&mut line)?;
    serde_json::from_str(line.trim_end())
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}

/// Opens its own connection and sends one long-poll Tail request, calling
/// `on_line` for each Response the server streams back as new lines are
/// ingested. Returns once the connection closes (the server side stops, or
/// the caller drops the returned reader by some other path -- this function
/// blocks for the life of the tail).
pub fn tail(
    endpoint: &Path,
    stream_name: &str,
    since: u64,
    mut on_line: impl FnMut(Response),
) -> std::io::Result<()> {
    let mut stream = Stream::connect(endpoint)?;
    writeln!(
        stream,
        "{}",
        serde_json::to_string(&Request::Tail {
            stream: stream_name.to_string(),
            since,
        })
        .unwrap()
    )?;
    let mut reader = BufReader::new(stream);
    loop {
        let mut line = String::new();
        if reader.read_line(&mut line)? == 0 {
            return Ok(());
        }
        match serde_json::from_str(line.trim_end()) {
            Ok(response) => on_line(response),
            Err(_) => continue,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::process::{Child, Command};
    use std::time::Duration;

    /// A unix domain socket's sun_path has a small platform-defined limit
    /// (104 bytes on macOS, 108 on Linux). macOS's own $TMPDIR is already
    /// ~49 bytes before anything of this test's own naming is added
    /// (github-ci-runners.md "$TMPDIR is long enough to break Unix
    /// sockets"), so plain /tmp is used directly on unix instead --
    /// matching planning-server/src/endpoint.rs's own short_root, and
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

    struct Server {
        child: Child,
        endpoint: PathBuf,
        dir: PathBuf,
    }

    impl Server {
        fn start() -> Self {
            let dir = short_temp_dir().join(format!(
                "tailpipe-client-flow-{}-{}",
                std::process::id(),
                unique()
            ));
            std::fs::create_dir_all(&dir).unwrap();
            let endpoint = dir.join("tailpipe.sock");
            let snapshot_dir = dir.join("snapshots");

            let child = Command::new(ensure_built(&sibling_bin_dir(), "tailpipe-server-rs"))
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

    /// Workspace sibling binaries all land in the same target/{debug,release}
    /// directory this test binary itself was built into.
    fn sibling_bin_dir() -> PathBuf {
        let mut dir = std::env::current_exe().expect("current test binary path");
        dir.pop();
        if dir.file_name().is_some_and(|part| part == "deps") {
            dir.pop();
        }
        dir
    }

    /// Builds `name` into `bin_dir` if it is not there yet, mirroring
    /// src/planning-server/tests/integration.rs's own `ensure_built`:
    /// tailpipe-server-rs is a Cargo *library* dependency of this crate (for
    /// the shared protocol types), not a bin/artifact dependency, so `cargo
    /// test -p tailpipe-client-rs` run alone builds its library but gives no
    /// guarantee its [[bin]] target already exists in the shared target dir
    /// -- observed for real in CI ("No such file or directory" starting
    /// tailpipe-server-rs) but never locally, where a prior full build had
    /// already staged it.
    fn ensure_built(bin_dir: &std::path::Path, name: &str) -> PathBuf {
        let program = bin_dir.join(format!("{name}{}", std::env::consts::EXE_SUFFIX));
        if program.is_file() {
            return program;
        }
        let mut cmd = Command::new(env!("CARGO"));
        cmd.arg("build").arg("-p").arg(name);
        // bin_dir is target/debug (native) or target/<triple>/debug
        // (cross-compiled); an explicit --target is required in the second
        // case or this build would land in target/debug instead, right
        // where bin_dir does NOT point. Walk up from bin_dir past whatever
        // sits above "target" -- one level native, two cross-compiled -- to
        // find the workspace root cargo must run from for its own default
        // output location to match bin_dir.
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
        // One build at a time: tests run on parallel threads and two of them
        // asking for the same sibling would otherwise race to build it.
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
            // Cargo reports where it actually put the binary; trust that
            // over the target/<triple>/debug layout assumed above.
            let stdout = String::from_utf8_lossy(&output.stdout);
            let built = stdout
                .lines()
                .filter_map(|line| serde_json::from_str::<serde_json::Value>(line).ok())
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

    fn unique() -> u64 {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    #[test]
    fn connect_and_request_gets_back_the_expected_response_for_every_verb() {
        let server = Server::start();

        let id = match connect_and_request(
            &server.endpoint,
            &Request::Ingest {
                stream: "s".into(),
                line: "hello".into(),
            },
        )
        .unwrap()
        {
            Response::Ingested { id } => id,
            other => panic!("unexpected response: {other:?}"),
        };

        match connect_and_request(&server.endpoint, &Request::List).unwrap() {
            Response::Streams { names } => assert_eq!(names, vec!["s".to_string()]),
            other => panic!("unexpected response: {other:?}"),
        }

        match connect_and_request(
            &server.endpoint,
            &Request::Read {
                stream: "s".into(),
                from: id,
                to: id,
            },
        )
        .unwrap()
        {
            Response::Lines { lines } => assert_eq!(lines[0].text, "hello"),
            other => panic!("unexpected response: {other:?}"),
        }

        match connect_and_request(
            &server.endpoint,
            &Request::Search {
                stream: "s".into(),
                mode: tailpipe_server_rs::protocol::SearchMode::Exact,
                query: "hello".into(),
            },
        )
        .unwrap()
        {
            Response::Lines { lines } => assert_eq!(lines.len(), 1),
            other => panic!("unexpected response: {other:?}"),
        }

        match connect_and_request(&server.endpoint, &Request::Save { stream: "s".into() }).unwrap()
        {
            Response::Saved { path } => assert!(std::path::Path::new(&path).exists()),
            other => panic!("unexpected response: {other:?}"),
        }
    }

    #[test]
    fn tail_receives_a_line_ingested_by_a_second_concurrent_connection() {
        let server = Server::start();
        let endpoint = server.endpoint.clone();

        let (sender, receiver) = std::sync::mpsc::channel();
        let tail_endpoint = endpoint.clone();
        std::thread::spawn(move || {
            let _ = tail(&tail_endpoint, "s", 0, move |response| {
                let _ = sender.send(response);
            });
        });

        // Give the tail connection a moment to be sent before the ingest,
        // so this proves live delivery rather than a startup-order fluke.
        std::thread::sleep(Duration::from_millis(100));
        connect_and_request(
            &endpoint,
            &Request::Ingest {
                stream: "s".into(),
                line: "pushed while tailing".into(),
            },
        )
        .unwrap();

        let response = receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("tail never delivered the ingested line");
        match response {
            Response::Lines { lines } => assert_eq!(lines[0].text, "pushed while tailing"),
            other => panic!("unexpected response: {other:?}"),
        }
    }
}
