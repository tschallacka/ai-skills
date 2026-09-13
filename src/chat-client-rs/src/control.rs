// MODE: DEV
// PACKAGE: PROD
//! The control socket: one client session owns the connection, and the rest of
//! the verbs borrow it.
//!
//! A `tail` holds a TLS connection for as long as it runs. A second connection
//! under the same nick is renamed by the server, so any other verb opening its
//! own would speak as `<nick>-2` and write the channel cursor in parallel with
//! the tail.
//!
//! So the tail is the session's owner. It binds a socket, and the other verbs
//! look there first and forward the request when it answers. One connection and
//! one cursor writer: there is never a second registration to collide with.
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
// T111: the queue/deadline/record machinery below is shared by both
// transports -- the unix arm's own UnixListener/UnixStream code and the
// platform-neutral TCP transport (`tcp_serve`/`tcp_ask`/`tcp_stop`, used by
// non-unix builds and exercised directly by this file's own tests on every
// platform, since std::net works the same everywhere). Only the choice of
// *which* socket type serve()/ask() end up using is platform-gated, in the
// two `mod imp` blocks below. On a unix build the TCP functions are reached
// only from tests (real callers go through the unix `mod imp` instead), which
// is exactly the point -- real coverage here, not only on Windows CI -- but
// it does mean rustc sees them as unreachable from a unix build's own
// non-test code.
#![cfg_attr(unix, allow(dead_code))]

use crate::control_auth;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
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
/// Exit code the TCP arm's owner answers with when a connection's challenge
/// proof does not verify. The asking side treats this the same as any other
/// reason the borrow did not work (None, fall back to an independent
/// connection) rather than surfacing it as a real answer -- see `tcp_ask`.
const EX_AUTH_FAILED: i32 = 77;

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
    /// compares rather than trusts, so a stale caller falls back instead of
    /// speaking as someone else.
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
    /// The TCP arm's shared secret (T111): empty on unix, where the socket's
    /// own file permissions are the guard. Never sent in the clear -- every
    /// connection proves it knows this via an HMAC challenge/response
    /// instead (`control_auth`), the same shape `ai_text_editor::auth` uses
    /// for its own TCP fallback.
    #[serde(default)]
    pub auth_token: String,
}

/// Where the record lives. A borrower can derive this from its own arguments
/// alone, which is why the record is the fixed point and names the socket.
pub fn owner_dir(state_dir: &Path) -> PathBuf {
    state_dir.join("owners")
}

pub fn record_path(state_dir: &Path, key: &str) -> PathBuf {
    owner_dir(state_dir).join(format!("{}.json", key))
}

/// Where the socket lives: the runtime directory when there is one, which is
/// short, already 0700, and cleared when the session ends. The state directory
/// is the fallback for where `XDG_RUNTIME_DIR` is unset, which includes macOS.
pub fn socket_dir(state_dir: &Path) -> PathBuf {
    socket_dir_from(std::env::var_os("XDG_RUNTIME_DIR").as_deref(), state_dir)
}

/// The same decision without reading the environment: these tests run as
/// parallel threads in one process, where an env write lands under a sibling.
fn socket_dir_from(runtime: Option<&std::ffi::OsStr>, state_dir: &Path) -> PathBuf {
    match runtime {
        Some(runtime) if !runtime.is_empty() && Path::new(runtime).is_dir() => {
            PathBuf::from(runtime).join("tsch-ai-skills-chat")
        }
        _ => owner_dir(state_dir),
    }
}

/// Distinguishes one state directory from another in a SHARED runtime
/// directory, where the session key alone would collide and two deliberately
/// isolated agents would each read the other as their own live owner.
fn state_tag(state_dir: &Path) -> String {
    // FNV-1a, as the session-key ladder uses for the same job: a collision
    // costs a declined takeover, not a security property.
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in state_dir.as_os_str().as_encoded_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100_0000_01b3);
    }
    format!("{:08x}", (hash >> 32) as u32)
}

pub fn socket_path(state_dir: &Path, key: &str) -> PathBuf {
    socket_dir(state_dir).join(format!("{}-{}.sock", key, state_tag(state_dir)))
}

/// A unix socket address is a fixed-size buffer -- 104 bytes on macOS, 108 on
/// Linux -- and an over-long path is TRUNCATED rather than refused, so binding
/// appears to work while every connect goes somewhere else.
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
    /// Requests accepted and not yet answered. The owner must not exit while
    /// one is outstanding, or the client reads EOF and reports an action that
    /// succeeded as one that may not have been performed.
    in_flight: AtomicUsize,
}

/// The owner's handle on its socket: hold it for as long as the connection is
/// held, then `stop()` it.
pub struct Control {
    inner: Arc<Inner>,
    socket: PathBuf,
    record: PathBuf,
    /// The inode this process bound. Only that inode is ever unlinked, so a
    /// later owner that has already replaced the path keeps its socket. A
    /// unix-only concept -- the TCP arm sets it to 0 and never reads it, since
    /// there is no path to unlink, only a record file `tcp_stop` removes
    /// directly.
    #[cfg_attr(not(unix), allow(dead_code))]
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
/// monitor's -- until the tail loop answers or `timeout` passes. The deadline
/// answer says the request MAY have been performed, never that it was not.
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
        in_flight: AtomicUsize::new(0),
    })
}

/// An explicit `--nick` or `--server` that disagrees with the owner asks to
/// be someone else, which borrowing the owner's connection cannot honour.
/// Empty means "whatever the session says", which the owner already is.
/// Shared by both transports -- purely a comparison, nothing socket-specific.
fn identity_matches(record: &OwnerRecord, request: &Request) -> bool {
    if !request.nick.is_empty() && request.nick != record.nick {
        return false;
    }
    if !request.server.is_empty() && request.server != record.server {
        return false;
    }
    true
}

// ---- the loopback-TCP transport (T111) -------------------------------------
//
// Plain `std::net`, so this compiles and runs identically on every platform;
// it is not behind `cfg(not(unix))`. Only the *choice* of using it (over the
// unix arm's own UnixListener/UnixStream code) is platform-gated, in the two
// `mod imp` blocks below. That is deliberate: it is what lets this file's own
// tests exercise the real mechanism on any machine, including this one,
// rather than leaving it provably untested until it reaches Windows CI.

/// Bind loopback TCP on an ephemeral port, generate a shared secret, and
/// serve it from a new thread -- the same "None is not an error" contract
/// `imp::serve` has on unix: an unfittable state, a live owner, or a failed
/// bind all mean "no owner; the caller tails on its own connection instead".
fn tcp_serve(state_dir: &Path, key: &str, mut record: OwnerRecord) -> Option<Control> {
    let record_dir = owner_dir(state_dir);
    std::fs::create_dir_all(&record_dir).ok()?;
    let record_path = record_path(state_dir, key);
    // A record already there is either a live owner or a leftover -- ask,
    // rather than assume, the same way the unix arm asks the socket path.
    if let Ok(text) = std::fs::read_to_string(&record_path) {
        if let Ok(existing) = serde_json::from_str::<OwnerRecord>(&text) {
            if let Ok(addr) = existing.socket.parse::<std::net::SocketAddr>() {
                if std::net::TcpStream::connect(addr).is_ok() {
                    return None;
                }
            }
        }
    }
    let listener = std::net::TcpListener::bind(("127.0.0.1", 0)).ok()?;
    let port = listener.local_addr().ok()?.port();
    let secret = control_auth::nonce().ok()?;
    record.socket = format!("127.0.0.1:{port}");
    record.pid = std::process::id();
    record.started_at_ns = now_ns();
    record.auth_token = secret.clone();
    let json = serde_json::to_string_pretty(&record).ok()?;
    std::fs::write(&record_path, json).ok()?;
    let inner = new_inner();
    let started_at_ns = record.started_at_ns;
    spawn_tcp_monitor(listener, Arc::clone(&inner), secret, started_at_ns);
    Some(Control {
        inner,
        socket: PathBuf::from(record.socket),
        record: record_path,
        inode: 0,
    })
}

fn spawn_tcp_monitor(
    listener: std::net::TcpListener,
    inner: Arc<Inner>,
    secret: String,
    started_at_ns: u64,
) {
    std::thread::spawn(move || {
        for stream in listener.incoming() {
            if inner.shutdown.load(Ordering::Acquire) {
                break;
            }
            match stream {
                Ok(stream) => tcp_handle(&inner, stream, &secret, started_at_ns),
                Err(_) => continue,
            }
        }
        // No path to unlink, unlike unix: the listener (and its ephemeral
        // port) simply drops when this thread ends.
    });
}

fn tcp_handle(inner: &Arc<Inner>, stream: std::net::TcpStream, secret: &str, started_at_ns: u64) {
    inner.in_flight.fetch_add(1, Ordering::AcqRel);
    tcp_serve_connection(inner, stream, secret, started_at_ns);
    inner.in_flight.fetch_sub(1, Ordering::AcqRel);
}

/// One connection's whole exchange: challenge, verify the proof, then the
/// same read-request/await/answer logic the unix arm has. A connection that
/// fails the challenge is answered `EX_AUTH_FAILED` and dropped without ever
/// reading a request line -- a legitimate client always has the current
/// secret (it reads the same record file this serve() call just wrote), so
/// this path is only ever reached by a stale record or a wake-up self-connect
/// from `tcp_stop`, never a normal borrower.
fn tcp_serve_connection(
    inner: &Arc<Inner>,
    stream: std::net::TcpStream,
    secret: &str,
    started_at_ns: u64,
) {
    use std::io::{BufRead, BufReader, Write};
    let _ = stream.set_read_timeout(Some(CLIENT_TIMEOUT));
    let _ = stream.set_write_timeout(Some(CLIENT_TIMEOUT));
    let mut reader = BufReader::new(match stream.try_clone() {
        Ok(clone) => clone,
        Err(_) => return,
    });
    let Ok(nonce_b64) = control_auth::nonce() else {
        return;
    };
    let Ok(nonce_bytes) = control_auth::decode_nonce(&nonce_b64) else {
        return;
    };
    let challenge = serde_json::json!({"type": "challenge", "nonce": nonce_b64});
    let Ok(mut challenge_bytes) = serde_json::to_vec(&challenge) else {
        return;
    };
    challenge_bytes.push(b'\n');
    {
        let writer = reader.get_mut();
        if writer.write_all(&challenge_bytes).is_err() || writer.flush().is_err() {
            return;
        }
    }
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() || line.trim().is_empty() {
        return;
    }
    #[derive(Deserialize)]
    struct AuthMessage {
        nonce: String,
        proof: String,
    }
    let authenticated = serde_json::from_str::<AuthMessage>(line.trim())
        .ok()
        .and_then(|auth| {
            let decoded = control_auth::decode_nonce(&auth.nonce).ok()?;
            (decoded == nonce_bytes).then_some(auth)
        })
        .map(|auth| {
            control_auth::verify(secret.as_bytes(), &nonce_bytes, started_at_ns, &auth.proof)
        })
        .unwrap_or(false);
    if !authenticated {
        tcp_answer(
            reader.into_inner(),
            &Reply::fail(
                EX_AUTH_FAILED,
                "chat-client-rs: control connection authentication failed",
            ),
        );
        return;
    }
    line.clear();
    if reader.read_line(&mut line).is_err() || line.trim().is_empty() {
        return;
    }
    let request: Request = match serde_json::from_str(line.trim()) {
        Ok(request) => request,
        Err(error) => {
            tcp_answer(
                reader.into_inner(),
                &Reply::fail(65, format!("owner: {}", error)),
            );
            return;
        }
    };
    tcp_answer(
        reader.into_inner(),
        &await_reply(inner, request, REPLY_TIMEOUT),
    );
}

fn tcp_answer(mut stream: std::net::TcpStream, reply: &Reply) {
    use std::io::Write;
    if let Ok(mut json) = serde_json::to_vec(reply) {
        json.push(b'\n');
        let _ = stream.write_all(&json);
        let _ = stream.flush();
    }
}

fn tcp_stop(control: &Control) {
    control.inner.shutdown.store(true, Ordering::Release);
    let deadline = SystemTime::now() + CLIENT_TIMEOUT;
    while control.inner.in_flight.load(Ordering::Acquire) > 0 && SystemTime::now() < deadline {
        std::thread::sleep(Duration::from_millis(2));
    }
    // accept() is blocking, so the monitor thread has to be woken to see the
    // shutdown flag -- one local connect, no handshake attempted or expected.
    if let Some(addr) = control
        .socket
        .to_str()
        .and_then(|s| s.parse::<std::net::SocketAddr>().ok())
    {
        let _ = std::net::TcpStream::connect(addr);
    }
    let _ = std::fs::remove_file(&control.record);
}

/// Connect to the owner's TCP endpoint, complete the challenge, forward the
/// request, and return its answer. `None` covers every reason the borrow did
/// not work -- no record, a record for a different transport, a dead port, a
/// malformed challenge, or the owner rejecting the proof -- collapsing an
/// auth failure into exactly the same fallback as a socket nobody answers,
/// per this module's own "nothing regresses" contract.
fn tcp_ask(state_dir: &Path, key: &str, request: &Request) -> Option<Reply> {
    use std::io::{BufRead, BufReader, Write};
    let record: OwnerRecord =
        serde_json::from_str(&std::fs::read_to_string(record_path(state_dir, key)).ok()?).ok()?;
    if !identity_matches(&record, request) {
        return None;
    }
    if record.auth_token.is_empty() {
        return None;
    }
    let addr: std::net::SocketAddr = record.socket.parse().ok()?;
    let mut stream = std::net::TcpStream::connect(addr).ok()?;
    stream.set_read_timeout(Some(CLIENT_TIMEOUT)).ok()?;
    stream.set_write_timeout(Some(CLIENT_TIMEOUT)).ok()?;
    let mut reader = BufReader::new(stream.try_clone().ok()?);
    let mut line = String::new();
    if reader.read_line(&mut line).is_err() || line.trim().is_empty() {
        return None;
    }
    #[derive(Deserialize)]
    struct Challenge {
        #[serde(rename = "type")]
        kind: String,
        nonce: String,
    }
    let challenge: Challenge = serde_json::from_str(line.trim()).ok()?;
    if challenge.kind != "challenge" {
        return None;
    }
    let nonce_bytes = control_auth::decode_nonce(&challenge.nonce).ok()?;
    let proof = control_auth::proof(
        record.auth_token.as_bytes(),
        &nonce_bytes,
        record.started_at_ns,
    )
    .ok()?;
    let auth_line = serde_json::json!({"nonce": challenge.nonce, "proof": proof});
    let mut auth_bytes = serde_json::to_vec(&auth_line).ok()?;
    auth_bytes.push(b'\n');
    let mut request_bytes = serde_json::to_vec(request).ok()?;
    request_bytes.push(b'\n');
    stream.write_all(&auth_bytes).ok()?;
    stream.write_all(&request_bytes).ok()?;
    stream.flush().ok()?;
    line.clear();
    // Accepted then unanswered is NOT a fallback case: the owner may have
    // sent it, and retrying on our own connection could double the message --
    // same reasoning the unix arm's `ask` states for the same shape.
    if reader.read_line(&mut line).is_err() || line.trim().is_empty() {
        return Some(Reply::fail(
            EX_BUSY,
            "chat-client-rs: the session owner accepted the request and did not answer; it may or may not have been sent",
        ));
    }
    let reply: Reply = serde_json::from_str(line.trim()).ok()?;
    if reply.code == EX_AUTH_FAILED {
        return None;
    }
    Some(reply)
}

#[cfg(unix)]
mod imp {
    use super::*;
    use std::fs;
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::fs::{MetadataExt, PermissionsExt};
    use std::os::unix::net::{UnixListener, UnixStream};

    /// Bind the socket and serve it from a new thread. None is not an error --
    /// an unfittable path, a live owner, an unwritable directory, a failed bind
    /// -- and a tail that cannot own the socket still tails.
    pub fn serve(state_dir: &Path, key: &str, record: OwnerRecord) -> Option<Control> {
        // Two places now -- record beside the state, socket in the runtime dir
        // -- and 0700 on each before anything is placed in it, since the socket
        // carries the right to speak as this agent.
        let record_dir = owner_dir(state_dir);
        fs::create_dir_all(&record_dir).ok()?;
        fs::set_permissions(&record_dir, fs::Permissions::from_mode(0o700)).ok()?;
        let dir = socket_dir(state_dir);
        fs::create_dir_all(&dir).ok()?;
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
    /// time, so a slow client delays the queue but never the tail loop. If it
    /// ends the socket is REMOVED, since one that never replies hangs clients.
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
        // Counted for the whole exchange, released only once the answer has
        // been written. `stop` waits on this, so an owner cannot exit between
        // deciding a reply and delivering it.
        inner.in_flight.fetch_add(1, Ordering::AcqRel);
        serve_connection(inner, stream);
        inner.in_flight.fetch_sub(1, Ordering::AcqRel);
    }

    fn serve_connection(inner: &Arc<Inner>, stream: UnixStream) {
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
        // An answer already decided must still be delivered, or the client
        // reads EOF. Bounded by the client's own timeout: past that its answer
        // is worthless, and a client that stopped reading must not hold us.
        let deadline = SystemTime::now() + CLIENT_TIMEOUT;
        while control.inner.in_flight.load(Ordering::Acquire) > 0 && SystemTime::now() < deadline {
            std::thread::sleep(Duration::from_millis(2));
        }
        // accept() is blocking, so the thread has to be woken to see the flag.
        // Its own socket is the wake-up: one connect, no reply expected.
        let _ = UnixStream::connect(&control.socket);
        unlink_own(&control.socket, control.inode);
        let _ = fs::remove_file(&control.record);
    }

    /// Forward one request to the session's owner. None means "no owner" -- no
    /// record, no socket, nothing listening, or an identity that does not match
    /// -- and the caller then opens its own connection as before.
    pub fn ask(state_dir: &Path, key: &str, request: &Request) -> Option<Reply> {
        let record: OwnerRecord =
            serde_json::from_str(&fs::read_to_string(record_path(state_dir, key)).ok()?).ok()?;
        if !identity_matches(&record, request) {
            return None;
        }
        // The RECORD names the socket: owner and borrower can disagree about
        // XDG_RUNTIME_DIR, so a re-derived path would match only by
        // coincidence. The fallback covers a record that carries no path.
        let socket = if record.socket.is_empty() {
            socket_path(state_dir, key)
        } else {
            PathBuf::from(&record.socket)
        };
        let mut stream = UnixStream::connect(&socket).ok()?;
        stream.set_read_timeout(Some(CLIENT_TIMEOUT)).ok()?;
        stream.set_write_timeout(Some(CLIENT_TIMEOUT)).ok()?;
        let mut json = serde_json::to_vec(request).ok()?;
        json.push(b'\n');
        stream.write_all(&json).ok()?;
        stream.flush().ok()?;
        let mut line = String::new();
        // Accepted then unanswered is NOT a fallback case: the owner may have
        // sent it, and retrying on our own connection could double the message.
        if BufReader::new(&stream).read_line(&mut line).is_err() || line.trim().is_empty() {
            return Some(Reply::fail(
                EX_BUSY,
                "chat-client-rs: the session owner accepted the request and did not answer; it may or may not have been sent",
            ));
        }
        serde_json::from_str(line.trim()).ok()
    }
}

/// Windows has no unix socket (T111): loopback TCP with a token in the
/// record, the shape `ai-text-editor`'s own TCP fallback already uses. The
/// mechanism itself (`tcp_serve`/`tcp_ask`/`tcp_stop`, above) is plain
/// `std::net` and not platform-gated at all; only this choice of using it
/// over the unix arm's UnixListener/UnixStream code is.
#[cfg(not(unix))]
mod imp {
    use super::*;

    pub fn serve(state_dir: &Path, key: &str, record: OwnerRecord) -> Option<Control> {
        tcp_serve(state_dir, key, record)
    }

    pub fn stop(control: &Control) {
        tcp_stop(control)
    }

    pub fn ask(state_dir: &Path, key: &str, request: &Request) -> Option<Reply> {
        tcp_ask(state_dir, key, request)
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

    /// A SHORT scratch directory: everything under test here binds a unix
    /// socket, so following a deep TMPDIR would push the path past its own
    /// guard and `serve` would answer None to a test expecting a socket.
    fn tmp_dir(tag: &str) -> PathBuf {
        // A runtime directory is short, 0700, and cleared at session end, so a
        // socket left by a killed test cannot outlive the login. /tmp is the
        // fallback where it is unset, which includes macOS and most CI runners.
        let base = ["T_SOCKET_TMPDIR", "XDG_RUNTIME_DIR"]
            .iter()
            .filter_map(|name| std::env::var(name).ok())
            .find(|value| !value.is_empty() && Path::new(value).is_dir())
            .unwrap_or_else(|| "/tmp".to_string());
        // Short and unique: the pid keeps two concurrent cargo runs apart
        // without spending the bytes a descriptive name would.
        let dir = PathBuf::from(base).join(format!("cc{}-{}", std::process::id(), tag));
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
        // The FALLBACK branch is the only one a long path can reach, since a
        // runtime directory is short by construction. Driven through
        // socket_dir_from because these tests share one process's environment.
        let dir = tmp_dir("longpath");
        let deep = dir.join("x".repeat(120));
        let fallback = socket_dir_from(None, &deep).join("key.sock");
        assert!(
            !path_fits(&fallback),
            "a 120-character state path must exceed the address limit: {}",
            fallback.display()
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[cfg(unix)]
    #[test]
    fn the_socket_follows_the_runtime_directory_and_stays_unique_per_state_dir() {
        // Both halves of the relocation, and the second is the one that bit:
        // a shared runtime directory means the key alone no longer identifies
        // an owner, so two state dirs must still give two sockets.
        let runtime = tmp_dir("rt");
        let a = tmp_dir("sa");
        let b = tmp_dir("sb");
        let chosen = socket_dir_from(Some(runtime.as_os_str()), &a);
        assert!(
            chosen.starts_with(&runtime),
            "a runtime directory must win over the state dir: {}",
            chosen.display()
        );
        assert_eq!(
            socket_dir_from(None, &a),
            owner_dir(&a),
            "with no runtime directory the socket stays beside the state"
        );
        assert_ne!(
            socket_path(&a, "same-key"),
            socket_path(&b, "same-key"),
            "one key under two state dirs must not collide on one socket"
        );
        for dir in [&runtime, &a, &b] {
            let _ = std::fs::remove_dir_all(dir);
        }
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

    // ---- the loopback-TCP transport (T111) ---------------------------------
    //
    // Calls tcp_serve/tcp_ask/tcp_stop directly rather than through serve/ask/
    // stop: on this (unix) machine those dispatch to the unix arm, but
    // std::net works identically everywhere, so exercising the TCP functions
    // by name gives real coverage of the mechanism Windows CI is the only
    // place that reaches through imp::serve/imp::ask instead.

    #[test]
    fn a_tcp_served_request_completes_the_challenge_and_reaches_the_owner() {
        let dir = tmp_dir("tcp-round-trip");
        let record = OwnerRecord {
            nick: "owner".into(),
            server: "127.0.0.1:1".into(),
            chan: "#ops".into(),
            ..OwnerRecord::default()
        };
        let control = tcp_serve(&dir, "k", record).expect("tcp must bind on loopback");
        assert!(
            !control.socket.to_string_lossy().is_empty(),
            "the record must carry the bound address"
        );
        let asked = std::thread::spawn({
            let dir = dir.clone();
            move || {
                tcp_ask(
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
        let reply = asked
            .join()
            .unwrap()
            .expect("the owner answered over the authenticated connection");
        assert_eq!(reply.code, 0);
        assert_eq!(reply.out, vec!["alice".to_string(), "bob".to_string()]);
        tcp_stop(&control);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_tcp_connection_with_no_proof_is_rejected_and_the_client_falls_back() {
        // A raw connection that never completes the challenge -- standing in
        // for a stray process finding the port without ever reading the
        // record's secret. tcp_ask always completes the real handshake, so
        // this drives the wire directly to prove the owner itself refuses.
        let dir = tmp_dir("tcp-no-proof");
        let control = tcp_serve(&dir, "k", OwnerRecord::default()).expect("tcp must bind");
        let addr = control
            .socket
            .to_str()
            .unwrap()
            .parse::<std::net::SocketAddr>()
            .unwrap();
        use std::io::{BufRead, BufReader, Write};
        let mut stream = std::net::TcpStream::connect(addr).unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        assert!(
            line.contains("challenge"),
            "must open with a challenge: {line}"
        );
        // Garbage instead of a real auth message.
        stream.write_all(b"not json at all\n").unwrap();
        stream.flush().unwrap();
        line.clear();
        reader.read_line(&mut line).unwrap();
        let reply: Reply = serde_json::from_str(line.trim()).unwrap();
        assert_eq!(reply.code, EX_AUTH_FAILED);
        tcp_stop(&control);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_second_tcp_owner_does_not_take_a_live_port_and_a_dead_one_is_reclaimed() {
        let dir = tmp_dir("tcp-second-owner");
        let first = tcp_serve(&dir, "k", OwnerRecord::default()).expect("first owner binds");
        assert!(
            tcp_serve(&dir, "k", OwnerRecord::default()).is_none(),
            "a live port must not be rebound: the second tail just tails"
        );
        tcp_stop(&first);
        let third = tcp_serve(&dir, "k", OwnerRecord::default())
            .expect("a dead port's record is reclaimed");
        tcp_stop(&third);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_tcp_request_for_another_identity_is_declined_so_the_caller_falls_back() {
        let dir = tmp_dir("tcp-identity");
        let record = OwnerRecord {
            nick: "owner".into(),
            server: "127.0.0.1:1".into(),
            ..OwnerRecord::default()
        };
        let control = tcp_serve(&dir, "k", record).expect("owner binds");
        assert!(tcp_ask(
            &dir,
            "k",
            &Request {
                verb: "send".into(),
                nick: "someone-else".into(),
                ..Request::default()
            }
        )
        .is_none());
        tcp_stop(&control);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn asking_tcp_with_no_owner_is_none_not_an_error() {
        let dir = tmp_dir("tcp-no-owner");
        assert!(tcp_ask(&dir, "k", &Request::default()).is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_owner_that_never_answers_reports_busy_rather_than_success() {
        // No tail loop, which is what a tail blocked on a long read looks like
        // from here. Driven through await_reply with a short timeout: the
        // property is the deadline, not how long a caller is willing to wait.
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
