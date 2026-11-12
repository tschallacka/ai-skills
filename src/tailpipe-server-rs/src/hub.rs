// MODE: DEV
// PACKAGE: PROD
//! The stream-name-to-StreamLog registry, shared across the thread-per-
//! connection model W08's main.rs runs, mirroring src/chat-server-rs/src/
//! main.rs's own `fn serve(peer: Arc<Peer>, hub: Arc<Hub>, ...)` pattern:
//! `Arc<Hub>` is cloned into each connection's own thread, and `Hub` itself
//! wraps the shared state in a `Mutex`.
//!
//! `dispatch` serves the five single-round-trip requests (Ingest/List/Read/
//! Search/Save). `Tail` is handled separately by `tail_since`, which blocks
//! on a condition variable an ingest signals, so the mutex is released for
//! the whole of its wait and an idle tail does not poll.

use crate::protocol::{Line, Request, Response};
use crate::snapshot::write_gzip_snapshot;
use crate::store::StreamLog;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Condvar, Mutex};
use std::time::Duration;

/// The longest an idle tail sleeps before it wakes to ask whether its
/// client is still there (GH #197). It bounds how long a gone client's
/// thread outlives it, and is how often an idle tail wakes at all.
pub const TAIL_LIVENESS_INTERVAL: Duration = Duration::from_millis(500);

pub struct Hub {
    streams: Mutex<HashMap<String, StreamLog>>,
    /// Signalled after every ingest; tails wait on it with `streams`.
    ingested: Condvar,
    snapshot_dir: PathBuf,
}

impl Hub {
    pub fn new(snapshot_dir: PathBuf) -> Self {
        Hub {
            streams: Mutex::new(HashMap::new()),
            ingested: Condvar::new(),
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
                drop(streams);
                self.ingested.notify_all();
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

    /// Follows `stream` past `since`, calling `on_line` for each new line,
    /// until `on_line` returns false (its client can no longer be written
    /// to) or `should_stop` returns true. `should_stop` is asked before every
    /// wait, and an idle tail waits at most `TAIL_LIVENESS_INTERVAL`, so a
    /// tail whose client has gone ends even when nothing is ever written.
    /// The lock is held only to read; the wait releases it.
    pub fn tail_since(
        &self,
        stream: &str,
        mut since: u64,
        mut on_line: impl FnMut(Line) -> bool,
        mut should_stop: impl FnMut() -> bool,
    ) {
        let pending = |streams: &mut HashMap<String, StreamLog>, since: u64| match streams
            .get_mut(stream)
        {
            Some(log) => log.read_range(since + 1, u64::MAX),
            None => Vec::new(),
        };
        while !should_stop() {
            let new_lines = {
                let mut streams = self.streams.lock().unwrap();
                let mut lines = pending(&mut streams, since);
                if lines.is_empty() {
                    streams = self
                        .ingested
                        .wait_timeout(streams, TAIL_LIVENESS_INTERVAL)
                        .unwrap()
                        .0;
                    lines = pending(&mut streams, since);
                }
                lines
            };
            for line in new_lines {
                since = since.max(line.id);
                if !on_line(line) {
                    return;
                }
            }
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
                    true
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

        // A bounded poll rather than a fixed sleep: an ingest wakes the
        // tail at once, so this is a generous ceiling for the healthy
        // case, not a bet against CI-runner scheduling contention
        // (github-ci-runners.md "a readiness budget is a ceiling, not a
        // sleep" -- a fixed 100ms wait here was observed failing for real
        // on a contended macOS CI runner, received still 0, not because
        // the line was lost but because the tailer thread had not yet been
        // scheduled to notice it).
        for _ in 0..200 {
            if received.load(Ordering::SeqCst) >= 1 {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        stop.store(true, Ordering::SeqCst);
        tailer.join().unwrap();

        assert_eq!(received.load(Ordering::SeqCst), 1);
    }

    /// GH #197: a tail whose writes fail must end, not keep a thread.
    #[test]
    fn a_tail_ends_when_its_client_can_no_longer_be_written_to() {
        let hub = Arc::new(Hub::new(scratch("write-fails")));
        for n in 0..3 {
            hub.dispatch(Request::Ingest {
                stream: "s".into(),
                line: format!("line {n}"),
            });
        }
        let calls = AtomicUsize::new(0);
        // A backstop, so a regression fails the assertion below instead of
        // hanging the run: without it a tail that ignores the failed write
        // would never stop at all.
        let backstop = std::time::Instant::now() + Duration::from_secs(2);
        hub.tail_since(
            "s",
            0,
            |_line| {
                calls.fetch_add(1, Ordering::SeqCst);
                false
            },
            || std::time::Instant::now() >= backstop,
        );
        assert_eq!(calls.load(Ordering::SeqCst), 1, "it stopped at the first failed write");
    }

    /// GH #197: a tail on a stream that never gets a line must still ask
    /// whether its client is there, rather than waiting forever.
    #[test]
    fn an_idle_tail_still_asks_whether_to_stop() {
        let hub = Hub::new(scratch("idle-stop"));
        let asked = AtomicUsize::new(0);
        let started = std::time::Instant::now();
        hub.tail_since(
            "never-written",
            0,
            |_line| true,
            || asked.fetch_add(1, Ordering::SeqCst) >= 2,
        );
        assert_eq!(asked.load(Ordering::SeqCst), 3);
        assert!(
            started.elapsed() < TAIL_LIVENESS_INTERVAL * 4,
            "two idle waits took {:?}",
            started.elapsed()
        );
    }

    /// The wait is on the ingest signal, not a sleep: a line arrives long
    /// before the liveness interval would have woken the tail.
    #[test]
    fn an_ingest_wakes_a_waiting_tail_at_once() {
        let hub = Arc::new(Hub::new(scratch("wake")));
        let hub_tail = Arc::clone(&hub);
        let tailer = std::thread::spawn(move || {
            let mut woke = None;
            let started = std::time::Instant::now();
            hub_tail.tail_since(
                "s",
                0,
                |_line| {
                    woke = Some(started.elapsed());
                    false
                },
                || false,
            );
            woke.unwrap()
        });
        std::thread::sleep(Duration::from_millis(100));
        hub.dispatch(Request::Ingest {
            stream: "s".into(),
            line: "wake up".into(),
        });
        let woke = tailer.join().unwrap();
        assert!(
            woke < Duration::from_millis(100) + TAIL_LIVENESS_INTERVAL / 2,
            "the tail only noticed the line after {woke:?}"
        );
    }
}
