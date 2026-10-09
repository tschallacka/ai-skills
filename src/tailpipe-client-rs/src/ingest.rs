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

    struct Server {
        child: Child,
        endpoint: PathBuf,
        dir: PathBuf,
    }

    impl Server {
        fn start() -> Self {
            let dir = std::env::temp_dir().join(format!(
                "tailpipe-client-ingest-{}-{}",
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
