// MODE: DEV
// PACKAGE: PROD
//! Reading piped stdin into a named stream, one Request::Ingest per line.

use crate::client::connect_and_request;
use std::io::BufRead;
use std::path::Path;
use tailpipe_server_rs::protocol::{Request, Response};

/// Reads `input` line by line, sending one Request::Ingest per line to
/// `stream` and printing each assigned id to `report` (stderr in
/// production) as it arrives, so a long-running piped command's progress is
/// visible rather than buffered until EOF.
pub fn run_ingest(
    endpoint: &Path,
    stream: &str,
    input: impl BufRead,
    mut report: impl FnMut(u64),
) -> std::io::Result<()> {
    for line in input.lines() {
        let line = line?;
        match connect_and_request(
            endpoint,
            &Request::Ingest {
                stream: stream.to_string(),
                line,
            },
        )? {
            Response::Ingested { id } => report(id),
            other => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("unexpected response to Ingest: {other:?}"),
                ))
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;
    use std::path::PathBuf;
    use std::process::{Child, Command};
    use std::sync::atomic::{AtomicU64, Ordering};
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
                "tailpipe-client-ingest-{}-{}",
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

    /// Builds `name` into `bin_dir` if it is not there yet -- see
    /// client.rs's own `ensure_built` (mirrored here, same reasoning:
    /// tailpipe-server-rs is a library dependency of this crate, not a
    /// bin/artifact one, so cargo gives no guarantee its [[bin]] exists yet
    /// when this crate's own tests run in isolation).
    fn ensure_built(bin_dir: &std::path::Path, name: &str) -> PathBuf {
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
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        COUNTER.fetch_add(1, Ordering::Relaxed)
    }

    #[test]
    fn run_ingest_sends_one_request_per_line_and_reports_each_assigned_id() {
        let server = Server::start();
        let input = Cursor::new(b"first\nsecond\nthird\n".to_vec());
        let mut reported = Vec::new();

        run_ingest(&server.endpoint, "s", input, |id| reported.push(id)).unwrap();

        assert_eq!(reported, vec![1, 2, 3]);
    }
}
