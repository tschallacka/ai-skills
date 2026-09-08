// MODE: DEV
// PACKAGE: PROD
//! The control socket: one client session owns the connection, and the rest of
//! the verbs borrow it.
//!
//! A `tail` holds a TLS connection open for as long as it runs. Every other
//! verb -- `send`, `read`, `names`, `join`, `leave` -- opened a SECOND
//! connection under the same nick, and the server, doing the RFC-correct thing
//! with a nick collision, suffixed it: an agent tailing as `aiskills` had its
//! own messages arrive from `aiskills-2` (B283). Worse, two processes then both
//! wrote the channel cursor, so the reader's watermark and the sender's
//! disagreed (B254, B269).
//!
//! So the tail becomes the session's owner. It binds a socket in the session
//! state directory and serves the other verbs over it; they look for that
//! socket first and forward the request when it answers. One connection, one
//! cursor writer, and the collision cannot happen because there is never a
//! second registration to collide with.
//!
//! **Nothing regresses where no tail runs.** An absent or unanswered socket is
//! not an error: the verb opens its own connection exactly as before. That is
//! the whole fallback contract, and every guard here is written to fail towards
//! it rather than towards an error.
//!
//! ## Why the socket gets its own thread
//!
//! ```text
//!   client                monitor thread              tail loop
//!     |  connect + request     |                          |
//!     |----------------------->| push(Request, reply_tx)   |
//!     |                        | waiting.store(Release)    |
//!     |                        |                     atomic load(Acquire)
//!     |                        |                     take under the lock
//!     |                        |                     serve on the wire
//!     |                        | reply_rx.recv_timeout <---| reply_tx.send
//!     |<-----------------------| write reply              |
//! ```
//!
//! The tail loop must never block on socket I/O -- a client that connects and
//! then stops reading would otherwise stall the bus for everyone. So the
//! listener lives in its own thread and the handoff is a queue plus one atomic:
//! the loop pays a single relaxed-cost load per iteration and touches the mutex
//! only when that load says there is something to take. All blocking is on the
//! monitor side, including the wait for the reply, and that wait has a deadline:
//! when it expires the monitor answers "owner busy" itself, because an
//! unanswered send must never look delivered.
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// How long the monitor waits for the tail loop to answer before telling the
/// client the owner is busy. It must exceed the tail's own read timeout, since
/// an idle loop only comes back around when that timeout expires.
const REPLY_TIMEOUT: Duration = Duration::from_secs(20);
/// How long a client waits for the whole exchange. Longer than REPLY_TIMEOUT,
/// so the "owner busy" answer arrives as an answer rather than as a hang.
const CLIENT_TIMEOUT: Duration = Duration::from_secs(30);
/// Exit code for "the owner could not answer in time". Distinct from a wire
/// failure: the request may not have been attempted at all, so a caller can
/// retry it.
pub const EX_BUSY: i32 = 75;

/// One borrowed verb, as it crosses the socket. Flat and optional-free: a field
/// a verb does not use is simply empty, which keeps the wire readable by hand
/// and forward-compatible with a verb that later needs one more.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct Request {
    pub verb: String,
    #[serde(default)]
    pub chan: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub since: String,
    #[serde(default)]
    pub mentions: bool,
    /// What the caller believed the session's nick and server were. The owner
    /// compares rather than trusts: a request aimed at a different identity is
    /// refused, so a stale caller falls back instead of speaking as someone
    /// else.
    #[serde(default)]
    pub nick: String,
    #[serde(default)]
    pub server: String,
}

/// What the owner saw on the wire, in the shape the calling process needs to
/// reproduce it: the lines it would have printed, the message it would have
/// printed to stderr, and the status it would have exited with.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct Reply {
    pub code: i32,
    #[serde(default)]
    pub out: Vec<String>,
    #[serde(default)]
    pub err: String,
}

impl Reply {
    pub fn ok(out: Vec<String>) -> Reply {
        Reply {
            code: 0,
            out,
            err: String::new(),
        }
    }

    pub fn fail(code: i32, err: impl Into<String>) -> Reply {
        Reply {
            code,
            out: Vec::new(),
            err: err.into(),
        }
    }
}

/// Who owns the connection right now. Written by the owner beside its socket,
/// read by every borrower before it forwards anything.
///
/// `pid` and `started_at_ns` are for a human reading the directory, and for
/// telling one owner from a later process that happens to reuse its pid -- a
/// pid alone is reused, which is the trap `ai-text-editor`'s SessionRecord pairs
/// with a start time for the same reason. Liveness itself is NOT decided from
/// them: it is decided by connecting, because that is the only test that
/// answers the question actually being asked.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct OwnerRecord {
    pub socket: String,
    pub pid: u32,
    pub started_at_ns: u64,
    #[serde(default)]
    pub server: String,
    #[serde(default)]
    pub nick: String,
    #[serde(default)]
    pub chan: String,
}

/// The directory holding one socket and one record per session key. Its own
/// directory rather than the state root: it is 0700, and a socket is not
/// something to mix in with the JSON a person may want to read.
pub fn owner_dir(state_dir: &Path) -> PathBuf {
    state_dir.join("owners")
}

pub fn socket_path(state_dir: &Path, key: &str) -> PathBuf {
    owner_dir(state_dir).join(format!("{}.sock", key))
}

pub fn record_path(state_dir: &Path, key: &str) -> PathBuf {
    owner_dir(state_dir).join(format!("{}.json", key))
}

/// A unix socket address is a fixed-size buffer in the kernel -- 104 bytes on
/// macOS, 108 on Linux -- and a path over that limit is TRUNCATED rather than
/// refused, so binding appears to work and every connect goes somewhere else.
/// A long `--state` path is therefore a reason to skip owning the socket, not a
/// reason to fail: the caller keeps today's behaviour.
pub fn path_fits(path: &Path) -> bool {
    path.as_os_str().len() < 100
}

fn now_ns() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0)
}

/// One queued request and the channel its answer goes back down.
pub struct Pending {
    pub request: Request,
    reply: Sender<Reply>,
}

impl Pending {
    /// Answer it. A dropped receiver means the client hung up while the owner
    /// was working, which is the client's business and not a failure here.
    pub fn answer(self, reply: Reply) {
        let _ = self.reply.send(reply);
    }
}

struct Inner {
    queue: Mutex<VecDeque<Pending>>,
    /// True while the queue is non-empty. The tail loop reads this instead of
    /// locking, so an idle loop pays one atomic load per iteration.
    waiting: AtomicBool,
    shutdown: AtomicBool,
}

/// The owner's handle on its socket: hold it for as long as the connection is
/// held, then `stop()` it.
pub struct Control {
    inner: Arc<Inner>,
    socket: PathBuf,
    record: PathBuf,
    /// The inode this process bound. Only that inode is ever unlinked, so a
    /// later owner that has already replaced the path keeps its socket.
    inode: u64,
}

impl Control {
    /// Anything queued, taken in arrival order. Cheap when idle: one atomic
    /// load, and the mutex only once something is actually waiting.
    pub fn take_pending(&self) -> Vec<Pending> {
        if !self.inner.waiting.load(Ordering::Acquire) {
            return Vec::new();
        }
        let mut queue = match self.inner.queue.lock() {
            Ok(queue) => queue,
            // A poisoned mutex means a monitor thread panicked mid-push. The
            // tail keeps tailing: dropping the queue is better than dying with
            // it, and the client gets its deadline answer.
            Err(poisoned) => poisoned.into_inner(),
        };
        let taken: Vec<Pending> = queue.drain(..).collect();
        self.inner.waiting.store(false, Ordering::Release);
        taken
    }

    pub fn socket(&self) -> &Path {
        &self.socket
    }
}

/// Queue a request and block -- on the caller's thread, which is always the
/// monitor's -- until the tail loop answers or `timeout` passes.
///
/// The deadline is the whole point: a tail busy on a long wire read, or one
/// whose loop has stopped taking work, must still leave the client with an
/// answer. That answer says the request may not have been performed, because a
/// send reported as delivered when it was not is the one outcome nothing can
/// recover from.
fn await_reply(inner: &Arc<Inner>, request: Request, timeout: Duration) -> Reply {
    let (reply_tx, reply_rx): (Sender<Reply>, Receiver<Reply>) = mpsc::channel();
    {
        let mut queue = match inner.queue.lock() {
            Ok(queue) => queue,
            Err(poisoned) => poisoned.into_inner(),
        };
        queue.push_back(Pending {
            request,
            reply: reply_tx,
        });
    }
    // Released AFTER the push, so a loop that observes the flag is guaranteed
    // to see the entry it announces.
    inner.waiting.store(true, Ordering::Release);
    reply_rx.recv_timeout(timeout).unwrap_or_else(|_| {
        Reply::fail(
            EX_BUSY,
            "chat-client-rs: the session owner did not answer in time; the request may not have been performed",
        )
    })
}

fn new_inner() -> Arc<Inner> {
    Arc::new(Inner {
        queue: Mutex::new(VecDeque::new()),
        waiting: AtomicBool::new(false),
        shutdown: AtomicBool::new(false),
    })
}

#[cfg(unix)]
mod imp {
    use super::*;
    use std::fs;
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    use std::os::unix::net::{UnixListener, UnixStream};

    /// Bind the socket and serve it from a new thread, or return None and leave
    /// the caller with today's behaviour.
    ///
    /// None is returned for every reason that is not an error: the path does not
    /// fit an address, another owner already answers on it, the directory
    /// cannot be made private, the bind fails. A tail that cannot own the
    /// socket still tails.
    pub fn serve(state_dir: &Path, key: &str, record: OwnerRecord) -> Option<Control> {
        let dir = owner_dir(state_dir);
        fs::create_dir_all(&dir).ok()?;
        // 0700 before anything is placed in it: the socket carries the right to
        // speak as this agent, so the directory is the outer guard, exactly as
        // interactive-shell does for its input socket.
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).ok()?;
        let socket = socket_path(state_dir, key);
        if !path_fits(&socket) {
            return None;
        }
        // A socket already there is either a live owner or a leftover. Ask,
        // rather than assume: connecting is the only test that distinguishes
        // them, and unlinking a live owner's socket would strand its clients.
        if socket.exists() {
            if UnixStream::connect(&socket).is_ok() {
                return None;
            }
            fs::remove_file(&socket).ok()?;
        }
        let listener = UnixListener::bind(&socket).ok()?;
        fs::set_permissions(&socket, fs::Permissions::from_mode(0o600)).ok()?;
        let inode = fs::metadata(&socket).map(|m| m.ino()).unwrap_or(0);
        write_record(state_dir, key, &socket, record);
        let inner = new_inner();
        spawn_monitor(listener, Arc::clone(&inner), socket.clone(), inode);
        Some(Control {
            inner,
            socket,
            record: record_path(state_dir, key),
            inode,
        })
    }

    fn write_record(state_dir: &Path, key: &str, socket: &Path, mut record: OwnerRecord) {
        record.socket = socket.display().to_string();
        record.pid = std::process::id();
        record.started_at_ns = now_ns();
        if let Ok(json) = serde_json::to_string_pretty(&record) {
            let path = record_path(state_dir, key);
            if fs::write(&path, json).is_ok() {
                let _ = fs::set_permissions(&path, fs::Permissions::from_mode(0o600));
            }
        }
    }

    /// The monitor thread: accept, queue, wait, answer. One connection at a
    /// time and in arrival order, so a slow client delays the queue behind it
    /// but never the tail loop.
    ///
    /// If it ends -- a panic, a listener that stops accepting -- the socket is
    /// REMOVED. A path that no longer answers sends every client back to its own
    /// connection; a path that answers and never replies would hang them all.
    fn spawn_monitor(listener: UnixListener, inner: Arc<Inner>, socket: PathBuf, inode: u64) {
        std::thread::spawn(move || {
            for stream in listener.incoming() {
                if inner.shutdown.load(Ordering::Acquire) {
                    break;
                }
                match stream {
                    Ok(stream) => handle(&inner, stream),
                    Err(_) => continue,
                }
            }
            if !inner.shutdown.load(Ordering::Acquire) {
                unlink_own(&socket, inode);
            }
        });
    }

    /// Read one request, queue it, and write back whatever the tail loop
    /// answers -- or the busy answer when the deadline passes first.
    fn handle(inner: &Arc<Inner>, stream: UnixStream) {
        let _ = stream.set_read_timeout(Some(CLIENT_TIMEOUT));
        let _ = stream.set_write_timeout(Some(CLIENT_TIMEOUT));
        let mut reader = BufReader::new(match stream.try_clone() {
            Ok(clone) => clone,
            Err(_) => return,
        });
        let mut line = String::new();
        if reader.read_line(&mut line).is_err() || line.trim().is_empty() {
            return;
        }
        let request: Request = match serde_json::from_str(line.trim()) {
            Ok(request) => request,
            Err(error) => {
                answer(stream, &Reply::fail(65, format!("owner: {}", error)));
                return;
            }
        };
        answer(stream, &await_reply(inner, request, REPLY_TIMEOUT));
    }

    fn answer(mut stream: UnixStream, reply: &Reply) {
        if let Ok(mut json) = serde_json::to_vec(reply) {
            json.push(b'\n');
            let _ = stream.write_all(&json);
            let _ = stream.flush();
        }
    }

    /// Unlink the path only while it still names the inode this process bound.
    /// A later owner replaces the path with its own socket, and removing that
    /// would break a bus that had already recovered.
    fn unlink_own(socket: &Path, inode: u64) {
        match fs::metadata(socket) {
            Ok(meta) if meta.ino() == inode => {
                let _ = fs::remove_file(socket);
            }
            _ => {}
        }
    }

    pub fn stop(control: &Control) {
        control.inner.shutdown.store(true, Ordering::Release);
        // accept() is blocking, so the thread has to be woken to see the flag.
        // Its own socket is the wake-up: one connect, no reply expected.
        let _ = UnixStream::connect(&control.socket);
        unlink_own(&control.socket, control.inode);
        let _ = fs::remove_file(&control.record);
    }

    /// Forward one request to the session's owner, if there is one that answers.
    ///
    /// None means "no owner": no record, no socket, nothing listening, or an
    /// owner whose identity does not match what the caller was told to use. In
    /// every one of those the caller opens its own connection as before.
    pub fn ask(state_dir: &Path, key: &str, request: &Request) -> Option<Reply> {
        let record: OwnerRecord =
            serde_json::from_str(&fs::read_to_string(record_path(state_dir, key)).ok()?).ok()?;
        if !identity_matches(&record, request) {
            return None;
        }
        let socket = socket_path(state_dir, key);
        let mut stream = UnixStream::connect(&socket).ok()?;
        stream.set_read_timeout(Some(CLIENT_TIMEOUT)).ok()?;
        stream.set_write_timeout(Some(CLIENT_TIMEOUT)).ok()?;
        let mut json = serde_json::to_vec(request).ok()?;
        json.push(b'\n');
        stream.write_all(&json).ok()?;
        stream.flush().ok()?;
        let mut line = String::new();
        // A connection that accepted the request and then gave no answer is NOT
        // a fallback case: the owner may have sent it. Saying so is the only
        // honest outcome, because retrying on our own connection could double
        // the message.
        if BufReader::new(&stream).read_line(&mut line).is_err() || line.trim().is_empty() {
            return Some(Reply::fail(
                EX_BUSY,
                "chat-client-rs: the session owner accepted the request and did not answer; it may or may not have been sent",
            ));
        }
        serde_json::from_str(line.trim()).ok()
    }

    /// An explicit `--nick` or `--server` that disagrees with the owner is a
    /// request to be someone else, and borrowing the owner's connection cannot
    /// honour it. An empty field means "whatever the session says", which the
    /// owner already is.
    fn identity_matches(record: &OwnerRecord, request: &Request) -> bool {
        if !request.nick.is_empty() && request.nick != record.nick {
            return false;
        }
        if !request.server.is_empty() && request.server != record.server {
            return false;
        }
        true
    }
}

/// Windows has no unix socket. The shape it needs is loopback TCP with a token
/// in the record, as `ai-text-editor`'s transport already does -- and until that
/// is written and tested on Windows, every verb there keeps the behaviour it has
/// today rather than a socket nobody has run.
#[cfg(not(unix))]
mod imp {
    use super::*;

    pub fn serve(_state_dir: &Path, _key: &str, _record: OwnerRecord) -> Option<Control> {
        None
    }

    pub fn stop(_control: &Control) {}

    pub fn ask(_state_dir: &Path, _key: &str, _request: &Request) -> Option<Reply> {
        None
    }
}

/// Bind the session's control socket and serve it, or return None and leave the
/// caller tailing exactly as it does today.
pub fn serve(state_dir: &Path, key: &str, record: OwnerRecord) -> Option<Control> {
    imp::serve(state_dir, key, record)
}

/// Stop serving: wake the monitor, unlink this process's own socket, drop the
/// record.
pub fn stop(control: &Control) {
    imp::stop(control)
}

/// Ask the session's owner to perform a verb, or None when there is no owner to
/// ask.
pub fn ask(state_dir: &Path, key: &str, request: &Request) -> Option<Reply> {
    imp::ask(state_dir, key, request)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_dir(name: &str) -> PathBuf {
        let base = std::env::var("TMPDIR").unwrap_or_else(|_| "/tmp".to_string());
        let dir = PathBuf::from(base).join(format!("chat-control-{}", name));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_request_round_trips_through_the_wire_form() {
        let request = Request {
            verb: "send".into(),
            chan: "#ops".into(),
            text: "hello\nworld".into(),
            ..Request::default()
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(
            !json.contains('\n'),
            "a serialized request must stay one line: {}",
            json
        );
        assert_eq!(serde_json::from_str::<Request>(&json).unwrap(), request);
    }

    #[test]
    fn a_reply_omits_nothing_a_caller_needs() {
        let reply = Reply::fail(70, "boom");
        let back: Reply = serde_json::from_str(&serde_json::to_string(&reply).unwrap()).unwrap();
        assert_eq!(back.code, 70);
        assert_eq!(back.err, "boom");
        assert!(back.out.is_empty());
    }

    #[test]
    fn a_request_from_an_older_writer_still_parses() {
        // Every optional field defaults, so a verb that gains a field does not
        // make the previous shape unreadable -- the fallback would otherwise
        // turn a version skew into two connections again.
        let back: Reply = serde_json::from_str("{\"code\":0}").unwrap();
        assert_eq!(back, Reply::ok(Vec::new()));
        let request: Request = serde_json::from_str("{\"verb\":\"names\"}").unwrap();
        assert_eq!(request.verb, "names");
        assert!(request.chan.is_empty());
    }

    #[test]
    fn asking_with_no_owner_is_none_not_an_error() {
        let dir = tmp_dir("no-owner");
        assert!(ask(&dir, "k", &Request::default()).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn an_over_long_socket_path_is_declined_rather_than_truncated() {
        let dir = tmp_dir("long-path");
        let deep = dir.join("x".repeat(120));
        assert!(!path_fits(&socket_path(&deep, "key")));
        assert!(serve(&deep, "key", OwnerRecord::default()).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn a_served_request_reaches_the_owner_and_its_answer_reaches_the_client() {
        let dir = tmp_dir("round-trip");
        let record = OwnerRecord {
            nick: "owner".into(),
            server: "127.0.0.1:1".into(),
            chan: "#ops".into(),
            ..OwnerRecord::default()
        };
        let control = serve(&dir, "k", record).expect("the socket must bind in a short tmp path");
        // Stand in for the tail loop: poll the queue the way it does, one
        // atomic load at a time, and answer from what was actually queued.
        let asked = std::thread::spawn({
            let dir = dir.clone();
            move || {
                ask(
                    &dir,
                    "k",
                    &Request {
                        verb: "names".into(),
                        chan: "#ops".into(),
                        ..Request::default()
                    },
                )
            }
        });
        let deadline = SystemTime::now() + Duration::from_secs(10);
        loop {
            for pending in control.take_pending() {
                assert_eq!(pending.request.verb, "names");
                pending.answer(Reply::ok(vec!["alice".into(), "bob".into()]));
            }
            if asked.is_finished() || SystemTime::now() > deadline {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let reply = asked.join().unwrap().expect("the owner answered");
        assert_eq!(reply.code, 0);
        assert_eq!(reply.out, vec!["alice".to_string(), "bob".to_string()]);
        stop(&control);
        assert!(
            !control.socket().exists(),
            "stop must unlink its own socket"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn a_second_owner_does_not_take_a_live_socket() {
        let dir = tmp_dir("second-owner");
        let first = serve(&dir, "k", OwnerRecord::default()).expect("first owner binds");
        assert!(
            serve(&dir, "k", OwnerRecord::default()).is_none(),
            "a live socket must not be rebound: the second tail just tails"
        );
        assert!(first.socket().exists());
        stop(&first);
        // With the owner gone the path is a leftover, and the next tail may
        // have it. This is the recovery that keeps a crashed owner from
        // permanently disabling the socket.
        let third = serve(&dir, "k", OwnerRecord::default()).expect("a dead socket is reclaimed");
        stop(&third);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn a_request_for_another_identity_is_declined_so_the_caller_falls_back() {
        let dir = tmp_dir("identity");
        let record = OwnerRecord {
            nick: "owner".into(),
            server: "127.0.0.1:1".into(),
            ..OwnerRecord::default()
        };
        let control = serve(&dir, "k", record).expect("owner binds");
        // An explicit --nick naming someone else must NOT be answered by this
        // owner; None sends the caller to its own connection, which is the only
        // way to honour what it asked for.
        assert!(ask(
            &dir,
            "k",
            &Request {
                verb: "send".into(),
                nick: "someone-else".into(),
                ..Request::default()
            }
        )
        .is_none());
        assert!(ask(
            &dir,
            "k",
            &Request {
                verb: "send".into(),
                server: "10.0.0.1:6667".into(),
                ..Request::default()
            }
        )
        .is_none());
        stop(&control);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_owner_that_never_answers_reports_busy_rather_than_success() {
        // No tail loop: nothing ever calls take_pending, which is what a tail
        // blocked on a long wire read looks like from here. The deadline must
        // produce the answer, and it must NOT be a success -- a send that was
        // never performed reporting 0 is the one outcome nothing downstream can
        // recover from.
        //
        // Driven through await_reply with a short timeout rather than through
        // the socket with REPLY_TIMEOUT: the property under test is the
        // deadline, and a test that proves it by waiting 20 seconds is a test
        // that gets deleted.
        let inner = new_inner();
        let reply = await_reply(&inner, Request::default(), Duration::from_millis(20));
        assert_eq!(reply.code, EX_BUSY);
        assert!(
            reply.err.contains("may not have been performed"),
            "the busy answer must not imply the request was skipped: {}",
            reply.err
        );
    }

    #[test]
    fn a_queued_request_is_announced_by_the_flag_before_it_is_taken() {
        // The handoff contract: the flag is the only thing an idle tail loop
        // reads, so a queued request that leaves it false would sit unserved
        // until some unrelated message happened to arrive.
        let inner = new_inner();
        assert!(!inner.waiting.load(Ordering::Acquire));
        let queued = Arc::clone(&inner);
        std::thread::spawn(move || {
            await_reply(&queued, Request::default(), Duration::from_secs(5));
        });
        let deadline = SystemTime::now() + Duration::from_secs(5);
        while !inner.waiting.load(Ordering::Acquire) && SystemTime::now() < deadline {
            std::thread::sleep(Duration::from_millis(5));
        }
        assert!(
            inner.waiting.load(Ordering::Acquire),
            "a pushed request must set the flag the tail loop polls"
        );
    }
}
