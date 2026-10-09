// MODE: DEV
// PACKAGE: PROD
//! The stream-name-to-StreamLog registry, shared across the thread-per-
//! connection model W08's main.rs runs, mirroring src/chat-server-rs/src/
//! main.rs's own `fn serve(peer: Arc<Peer>, hub: Arc<Hub>, ...)` pattern:
//! `Arc<Hub>` is cloned into each connection's own thread, and `Hub` itself
//! wraps the shared state in a `Mutex`.
//!
//! `dispatch` serves the five single-round-trip requests (Ingest/List/Read/
//! Search/Save). `Tail` is handled separately by `tail_since`, which never
//! holds the mutex across its own wait: each poll iteration locks briefly,
//! takes whatever is new, and releases the lock again before sleeping.

use crate::protocol::{Line, Request, Response};
use crate::snapshot::write_gzip_snapshot;
use crate::store::StreamLog;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

pub struct Hub {
    streams: Mutex<HashMap<String, StreamLog>>,
    snapshot_dir: PathBuf,
}

impl Hub {
    pub fn new(snapshot_dir: PathBuf) -> Self {
        Hub {
            streams: Mutex::new(HashMap::new()),
            snapshot_dir,
        }
    }

    /// Dispatches one of the five single-round-trip requests. `Request::Tail`
    /// is not handled here -- see `tail_since`.
    pub fn dispatch(&self, request: Request) -> Response {
        match request {
            Request::Ingest { stream, line } => {
                let mut streams = self.streams.lock().unwrap();
                let id = streams.entry(stream).or_default().ingest(line);
                Response::Ingested { id }
            }
            Request::List => {
                let streams = self.streams.lock().unwrap();
                Response::Streams {
                    names: streams.keys().cloned().collect(),
                }
            }
            Request::Read { stream, from, to } => {
                let mut streams = self.streams.lock().unwrap();
                match streams.get_mut(&stream) {
                    Some(log) => Response::Lines {
                        lines: log.read_range(from, to),
                    },
                    None => Response::Error {
                        code: "not_found".into(),
                        message: format!("no such stream: {stream}"),
                    },
                }
            }
            Request::Search {
                stream,
                mode,
                query,
            } => {
                let mut streams = self.streams.lock().unwrap();
                match streams.get_mut(&stream) {
                    Some(log) => match log.search(&mode, &query) {
                        Ok(lines) => Response::Lines { lines },
                        Err(message) => Response::Error {
                            code: "bad_search".into(),
                            message,
                        },
                    },
                    None => Response::Error {
                        code: "not_found".into(),
                        message: format!("no such stream: {stream}"),
                    },
                }
            }
            Request::Save { stream } => {
                let streams = self.streams.lock().unwrap();
                match streams.get(&stream) {
                    Some(log) => match write_gzip_snapshot(&self.snapshot_dir, &stream, log) {
                        Ok(path) => Response::Saved {
                            path: path.display().to_string(),
                        },
                        Err(error) => Response::Error {
                            code: "snapshot_failed".into(),
                            message: error.to_string(),
                        },
                    },
                    None => Response::Error {
                        code: "not_found".into(),
                        message: format!("no such stream: {stream}"),
                    },
                }
            }
            Request::Tail { .. } => Response::Error {
                code: "wrong_dispatch".into(),
                message: "Tail is served by tail_since, not dispatch".into(),
            },
        }
    }

    /// Long-polls `stream` for lines past `since`, calling `on_line` for
    /// each as they appear, until `should_stop` reports true. Re-locks
    /// briefly per iteration; never holds the lock across the sleep.
    pub fn tail_since(
        &self,
        stream: &str,
        mut since: u64,
        mut on_line: impl FnMut(Line),
        should_stop: impl Fn() -> bool,
    ) {
        while !should_stop() {
            let new_lines = {
                let mut streams = self.streams.lock().unwrap();
                match streams.get_mut(stream) {
                    Some(log) => log.read_range(since + 1, u64::MAX),
                    None => Vec::new(),
                }
            };
            for line in new_lines {
                since = since.max(line.id);
                on_line(line);
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    pub fn sweep_idle(&self, idle_threshold: Duration) -> Vec<String> {
        let mut streams = self.streams.lock().unwrap();
        crate::eviction::sweep_idle_streams(&mut streams, idle_threshold, &self.snapshot_dir)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::Arc;

    fn scratch(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("tp-hub-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn an_ingest_creates_a_stream_and_read_search_dispatch_correctly() {
        let hub = Hub::new(scratch("dispatch"));
        let id = match hub.dispatch(Request::Ingest {
            stream: "s".into(),
            line: "hello world".into(),
        }) {
            Response::Ingested { id } => id,
            other => panic!("unexpected response: {other:?}"),
        };
        assert_eq!(id, 1);

        match hub.dispatch(Request::Read {
            stream: "s".into(),
            from: 1,
            to: 1,
        }) {
            Response::Lines { lines } => assert_eq!(lines[0].text, "hello world"),
            other => panic!("unexpected response: {other:?}"),
        }

        match hub.dispatch(Request::Search {
            stream: "s".into(),
            mode: crate::protocol::SearchMode::Exact,
            query: "hello".into(),
        }) {
            Response::Lines { lines } => assert_eq!(lines.len(), 1),
            other => panic!("unexpected response: {other:?}"),
        }
    }

    #[test]
    fn a_save_does_not_remove_the_stream() {
        let hub = Hub::new(scratch("save"));
        hub.dispatch(Request::Ingest {
            stream: "s".into(),
            line: "keep me".into(),
        });
        match hub.dispatch(Request::Save { stream: "s".into() }) {
            Response::Saved { path } => assert!(std::path::Path::new(&path).exists()),
            other => panic!("unexpected response: {other:?}"),
        }
        match hub.dispatch(Request::List) {
            Response::Streams { names } => assert_eq!(names, vec!["s".to_string()]),
            other => panic!("unexpected response: {other:?}"),
        }
    }

    #[test]
    fn dispatch_from_two_concurrent_connections_plus_a_sweep_does_not_deadlock_or_race() {
        let hub = Arc::new(Hub::new(scratch("concurrent")));
        let hub_a = Arc::clone(&hub);
        let hub_b = Arc::clone(&hub);

        let a = std::thread::spawn(move || {
            for n in 0..50 {
                hub_a.dispatch(Request::Ingest {
                    stream: "s".into(),
                    line: format!("a{n}"),
                });
            }
        });
        let b = std::thread::spawn(move || {
            for n in 0..50 {
                hub_b.dispatch(Request::Ingest {
                    stream: "s".into(),
                    line: format!("b{n}"),
                });
            }
        });
        let sweep_hub = Arc::clone(&hub);
        let sweep = std::thread::spawn(move || {
            for _ in 0..10 {
                sweep_hub.sweep_idle(Duration::from_secs(3600));
            }
        });

        a.join().unwrap();
        b.join().unwrap();
        sweep.join().unwrap();

        match hub.dispatch(Request::List) {
            Response::Streams { names } => assert_eq!(names, vec!["s".to_string()]),
            other => panic!("unexpected response: {other:?}"),
        }
    }

    #[test]
    fn tail_does_not_hold_the_lock_across_its_own_wait() {
        let hub = Arc::new(Hub::new(scratch("tail")));
        let first_id = match hub.dispatch(Request::Ingest {
            stream: "s".into(),
            line: "first".into(),
        }) {
            Response::Ingested { id } => id,
            other => panic!("unexpected response: {other:?}"),
        };

        let received = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let hub_tail = Arc::clone(&hub);
        let received_tail = Arc::clone(&received);
        let stop_tail = Arc::clone(&stop);
        let tailer = std::thread::spawn(move || {
            hub_tail.tail_since(
                "s",
                first_id,
                |_line| {
                    received_tail.fetch_add(1, Ordering::SeqCst);
                },
                || stop_tail.load(Ordering::SeqCst),
            );
        });

        // While the tail is pending, a second dispatch must complete
        // promptly -- proof the mutex is not held across the sleep.
        std::thread::sleep(Duration::from_millis(50));
        hub.dispatch(Request::Ingest {
            stream: "s".into(),
            line: "second".into(),
        });

        std::thread::sleep(Duration::from_millis(100));
        stop.store(true, Ordering::SeqCst);
        tailer.join().unwrap();

        assert_eq!(received.load(Ordering::SeqCst), 1);
    }
}
