// MODE: DEV
// PACKAGE: PROD
//! The RFC-1459-grammar TLS chat server.
//!
//! Speaks the standard IRC message grammar (see chat-proto) so a stock IRC
//! client that supports TLS can connect, register (NICK+USER), join, and
//! message. Adds one additive extension command, FETCH #chan <since>, for the
//! agent delta-tail workflow (history replay); a standard client never sends it.
//!
//! Communications are TLS-only (rustls, ring provider). The server mints a
//! self-signed certificate at first run via the openssl CLI and reuses it, so a
//! client that pins the cert (TOFU) stays stable across restarts.
//!
//! Storage layout mirrors the interpreter tiers: a channel is one log file,
//! `MSG <chan> <id> <ts> <nick> :<text>` per line, and the next id is
//! highest+1. Non-MSG lines are skipped, malformed ones cannot kill the
//! connection.

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chat_proto::message::{numeric, numerics, Message, FETCH_END};

// How long a socket write may block before the peer is declared unresponsive,
// and how much undelivered broadcast one peer may accumulate. Both are bounds
// on damage a single peer can do, not tuning knobs: without the first, a peer
// that stops reading blocks its own thread forever; without the second, the
// queue that keeps that thread from blocking anyone else grows without bound.
const WRITE_TIMEOUT: Duration = Duration::from_secs(5);
const OUTBOX_MAX_BYTES: usize = 1 << 20;

struct ConnState {
    conn: rustls::ServerConnection,
    tcp: TcpStream,
    nick: String,
    user: String,
    host: String,
    closed: bool,
}

impl ConnState {
    // Returns the write error rather than swallowing it: a peer that has
    // stopped reading fails here (SO_SNDTIMEO, set on accept), and the caller
    // must mark the connection closed so the teardown path runs. Swallowing it
    // meant a timed-out peer was retried on every later message forever.
    fn write_line(&mut self, s: &str) -> std::io::Result<()> {
        self.conn
            .writer()
            .write_all(format!("{}\r\n", s).as_bytes())?;
        // Write-only flush: complete_io would block reading for more input,
        // which deadlocks when the peer is also blocked reading a response.
        loop {
            match self.conn.write_tls(&mut self.tcp) {
                Ok(0) => return Ok(()),
                Ok(_) => continue,
                Err(e) => return Err(e),
            }
        }
    }
}

/// One connection's pending outbound lines, plus the channel membership a
/// broadcaster needs to decide whether to send at all.
///
/// This is a leaf lock: it is held only long enough to push or drain a Vec,
/// never across I/O and never while any other lock is taken. That is the whole
/// point of it (see `Peer`).
struct Outbox {
    queue: VecDeque<String>,
    bytes: usize,
    joined: Vec<String>,
    dead: bool,
}

/// What happened to a line offered to a peer.
#[derive(Debug, PartialEq, Eq)]
enum Offer {
    Queued,
    /// The peer is not in the channel, or is already finished.
    Skipped,
    /// The peer is not draining its queue; it has been marked dead so its own
    /// thread tears it down instead of being written to again.
    Dropped,
}

/// A live connection: the state its own thread does I/O on, and the outbox
/// every other thread talks to instead.
///
/// The split is the fix for B122. The broadcast path used to hold `hub.writers`
/// and then lock a second connection's state and write to its socket from the
/// sender's thread. Two things followed, both measured:
///
/// * A socket write blocks for as long as the peer declines to read, and it was
///   blocking while holding `hub.writers` -- which the accept loop must take to
///   register a new connection. So connects completed through the kernel
///   backlog and were then never serviced.
/// * Even with nothing blocking, the target's own thread holds its state across
///   a 200ms `read_tls`, so a broadcaster contending for it loses that race
///   almost every time. One idle subscriber was enough to starve the
///   broadcaster for tens of seconds while it held `hub.writers`.
///
/// Distinct mutexes were never what made that safe -- safety needs a lock
/// order, which the comment there claimed was unnecessary. Now there is one:
/// `writers` is taken alone, `slot` is taken only by the connection's own
/// thread, and `out` is a leaf. No socket I/O happens under a shared lock, so a
/// peer that refuses to read can no longer stall anything but itself.
struct Peer {
    slot: Mutex<Option<ConnState>>,
    out: Mutex<Outbox>,
}

impl Peer {
    fn new() -> Peer {
        Peer {
            slot: Mutex::new(None),
            out: Mutex::new(Outbox {
                queue: VecDeque::new(),
                bytes: 0,
                joined: Vec::new(),
                dead: false,
            }),
        }
    }

    /// Record which channels this connection is in, so a broadcaster can filter
    /// without touching the connection state.
    fn set_joined(&self, joined: &[String]) {
        if let Ok(mut o) = self.out.lock() {
            o.joined = joined.to_vec();
        }
    }

    /// Mark the connection finished. Idempotent, and safe to call from any
    /// thread: it only touches the leaf lock.
    fn mark_dead(&self) {
        if let Ok(mut o) = self.out.lock() {
            o.dead = true;
            o.queue.clear();
            o.bytes = 0;
        }
    }

    /// Offer one line to this peer. Never writes to a socket, so it cannot
    /// block on the peer's behaviour; the peer's own thread does the writing.
    fn offer(&self, chan: &str, line: &str) -> Offer {
        let mut o = match self.out.lock() {
            Ok(o) => o,
            // A poisoned outbox means the owning thread panicked; there is
            // nothing to deliver to.
            Err(_) => return Offer::Skipped,
        };
        if o.dead || !o.joined.iter().any(|j| j == chan) {
            return Offer::Skipped;
        }
        if o.bytes.saturating_add(line.len()) > OUTBOX_MAX_BYTES {
            o.dead = true;
            o.queue.clear();
            o.bytes = 0;
            return Offer::Dropped;
        }
        o.bytes += line.len();
        o.queue.push_back(line.to_string());
        Offer::Queued
    }

    /// Take everything queued for this connection, and whether it has been
    /// declared dead. Called by the connection's own thread.
    fn drain(&self) -> (Vec<String>, bool) {
        match self.out.lock() {
            Ok(mut o) => {
                o.bytes = 0;
                (o.queue.drain(..).collect(), o.dead)
            }
            Err(_) => (Vec::new(), true),
        }
    }
}

struct Hub {
    chan_dir: PathBuf,
    highest: Mutex<HashMap<String, u64>>,
    channels: Mutex<HashMap<String, Vec<String>>>, // chan -> nicks
    // Channel topics, in memory only: a standard IRC client sends TOPIC on
    // join and displays it; persistence is not part of the message bus.
    topics: Mutex<HashMap<String, String>>, // chan -> topic
    // Registered nicks, so a nick-in-use check never needs the writers lock
    // (holding a connection's own slot guard while acquiring writers is an
    // ABBA deadlock with the broadcast path).
    nicks: Mutex<HashMap<String, u64>>, // nick -> conn index
    // One peer per connection, keyed by index. Taken alone and held only long
    // enough to snapshot the list: nothing that can block is done under it.
    writers: Mutex<Vec<Arc<Peer>>>,
}

impl Hub {
    fn chan_path(&self, chan: &str) -> PathBuf {
        self.chan_dir.join(format!("{}.log", chan))
    }

    /// Release everything one connection held: its nick, its channel
    /// memberships, and its writer slot.
    ///
    /// Without this a disconnect left the nick registered for the life of the
    /// server, so the SECOND agent on a machine could never register that nick
    /// again. The nick is only surrendered when the registry still points at
    /// THIS connection index; a later connection that took it over keeps it.
    fn deregister(&self, nick: &str, idx: usize, chans: &[String]) {
        if !nick.is_empty() {
            if let Ok(mut nicks) = self.nicks.lock() {
                if nicks.get(nick).copied() == Some(idx as u64) {
                    nicks.remove(nick);
                }
            }
        }
        if let Ok(mut channels) = self.channels.lock() {
            for chan in chans {
                if let Some(members) = channels.get_mut(chan) {
                    members.retain(|m| m != nick);
                }
            }
        }
        // The peer itself stays in `writers` so live connections keep their
        // index (broadcast addresses members by index). Its connection state is
        // dropped by the owning thread and its outbox is marked dead, so it is
        // a few bytes rather than a socket, and the broadcast path skips it.
        //
        // The peer is fetched under `writers` and marked afterwards: taking the
        // outbox lock while holding `writers` would put two locks in one place
        // for no reason, and the point of this design is that there is only ever
        // one.
        let peer = self
            .writers
            .lock()
            .ok()
            .and_then(|writers| writers.get(idx).map(Arc::clone));
        if let Some(peer) = peer {
            peer.mark_dead();
        }
    }

    fn scan_highest(&self, chan: &str) -> u64 {
        let path = self.chan_path(chan);
        let mut top = 0u64;
        if let Ok(f) = fs::File::open(&path) {
            for line in BufReader::new(f).lines().map_while(Result::ok) {
                let f: Vec<&str> = line.splitn(4, ' ').collect();
                if f.len() >= 4 && f[0] == "MSG" {
                    if let Ok(id) = f[2].parse::<u64>() {
                        if id > top {
                            top = id;
                        }
                    }
                }
            }
        }
        if let Ok(mut h) = self.highest.lock() {
            let e = h.entry(chan.to_string()).or_insert(0);
            if top > *e {
                *e = top;
            }
        }
        top
    }

    fn append(&self, chan: &str, nick: &str, text: &str) -> std::io::Result<(u64, String)> {
        let path = self.chan_path(chan);
        fs::create_dir_all(&self.chan_dir)?;
        let lock = self.chan_dir.join(format!("{}.lock", chan));
        let mut tries = 0;
        while fs::create_dir(&lock).is_err() {
            if lock.join("pid").exists() {
                let _ = fs::remove_dir_all(&lock);
                continue;
            }
            tries += 1;
            if tries >= 200 {
                return Err(std::io::Error::other("lock timeout"));
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        let result = (|| -> std::io::Result<(u64, String)> {
            let last = self.scan_highest(chan);
            let id = last + 1;
            let ts = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let line = format!(
                "MSG {} {} {} {} :{}\n",
                chan,
                id,
                ts,
                nick,
                text.replace('\n', " ")
            );
            let mut f = fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)?;
            f.write_all(line.as_bytes())?;
            if let Ok(mut h) = self.highest.lock() {
                h.insert(chan.to_string(), id);
            }
            Ok((id, line))
        })();
        let _ = fs::remove_dir(&lock);
        result
    }

    fn fetch(&self, chan: &str, since: u64) -> Vec<String> {
        let path = self.chan_path(chan);
        let mut out = Vec::new();
        if let Ok(f) = fs::File::open(&path) {
            for line in BufReader::new(f).lines().map_while(Result::ok) {
                let f: Vec<&str> = line.splitn(4, ' ').collect();
                if f.len() >= 4 && f[0] == "MSG" && f[1] == chan {
                    if let Ok(id) = f[2].parse::<u64>() {
                        if id > since {
                            out.push(line);
                        }
                    }
                }
            }
        }
        out
    }

    /// The channel's current maximum message id (0 when empty). A client uses
    /// this to seed a cursor without reading the whole history.
    fn last_id(&self, chan: &str) -> u64 {
        self.scan_highest(chan)
    }

    /// Like `fetch`, but only rows whose text mentions `@nick` (server-side
    /// mention tracking, so a client can watch for its name without pulling
    /// the whole channel).
    fn fetch_mentions(&self, chan: &str, since: u64, nick: &str) -> Vec<String> {
        let needle = format!("@{}", nick);
        let path = self.chan_path(chan);
        let mut out = Vec::new();
        if let Ok(f) = fs::File::open(&path) {
            for line in BufReader::new(f).lines().map_while(Result::ok) {
                let f: Vec<&str> = line.splitn(4, ' ').collect();
                if f.len() >= 4 && f[0] == "MSG" && f[1] == chan {
                    if let Ok(id) = f[2].parse::<u64>() {
                        if id > since && line.contains(&needle) {
                            out.push(line);
                        }
                    }
                }
            }
        }
        out
    }
}

fn valid_chan(c: &str) -> bool {
    c.len() > 1
        && c.len() <= 33
        && c.starts_with('#')
        && c[1..]
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
}

// A umode set a client may send at connect or with /mode <self>: the letters
// a standard client expects to be accepted (i invisible, w WALLOPS, s server
// notices) with optional +/- prefixes. There is no behavior behind them; the
// point is that the set succeeds so the client's startup reads as clean.
fn valid_umode_set(flags: &str) -> bool {
    let body = flags.trim_start_matches(['+', '-']);
    !body.is_empty() && body.chars().all(|ch| matches!(ch, 'i' | 'w' | 's'))
}

fn valid_nick(n: &str) -> bool {
    (1..=32).contains(&n.len())
        && n.chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
}

fn ensure_cert(home: &Path) -> Result<(PathBuf, PathBuf), String> {
    let crt = home.join("server.crt");
    let key = home.join("server.key");
    if crt.exists() && key.exists() {
        return Ok((crt, key));
    }
    fs::create_dir_all(home).map_err(|e| format!("cannot create home: {}", e))?;
    // Mint a self-signed certificate in Rust (ring-backed) so the server needs
    // no external binary: no openssl, no PATH dependency.
    let cn = std::env::var("CHAT_CERT_CN").unwrap_or_else(|_| "localhost".into());
    let mut params = rcgen::CertificateParams::new(vec![cn.clone()])
        .map_err(|e| format!("cert params: {}", e))?;
    params
        .distinguished_name
        .push(rcgen::DnType::CommonName, cn);
    let key_pair = rcgen::KeyPair::generate().map_err(|e| format!("keypair: {}", e))?;
    let cert = params
        .self_signed(&key_pair)
        .map_err(|e| format!("self-signed: {}", e))?;
    fs::write(&crt, cert.pem()).map_err(|e| format!("write cert: {}", e))?;
    fs::write(&key, key_pair.serialize_pem()).map_err(|e| format!("write key: {}", e))?;
    Ok((crt, key))
}

fn server_config(crt: &Path, key: &Path) -> Result<rustls::ServerConfig, String> {
    use rustls_pki_types::pem::PemObject;
    let certs = rustls_pki_types::CertificateDer::pem_file_iter(crt)
        .map_err(|e| format!("read cert: {}", e))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("parse cert: {}", e))?;
    let key = rustls_pki_types::PrivateKeyDer::from_pem_file(key)
        .map_err(|e| format!("read key: {}", e))?;
    let cfg = rustls::ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| format!("configure rustls: {}", e))?;
    // No ALPN: a generic IRC-over-TLS client historically does not negotiate
    // one, and offering "irc" can cause a HandshakeFailure with some peers.
    // cfg.alpn_protocols = vec![b"irc".to_vec()];
    Ok(cfg)
}

struct Session {
    server_name: String,
    nick: String,
    user: String,
    host: String,
    registered: bool,
    joined: Vec<String>,
    closed: bool,
}

fn serve(peer: Arc<Peer>, hub: Arc<Hub>, idx: usize, server_name: String) {
    let mut raw = [0u8; 4096];
    let mut sess = Session {
        server_name,
        nick: String::new(),
        user: String::new(),
        host: "localhost".into(),
        joined: Vec::new(),
        registered: false,
        closed: false,
    };

    // Emit to a connection state's TLS writer. Called while holding the
    // slot guard, so it never re-locks the same slot (no deadlock). A write
    // that fails -- including a peer that stopped reading long enough to hit
    // the write timeout -- closes the connection, so the teardown below runs
    // instead of the peer being written to again on every later message.
    fn w(st: &mut ConnState, s: &str) {
        if !st.closed && st.write_line(s).is_err() {
            st.closed = true;
        }
    }

    // Set the moment this connection is finished -- a dead peer, a failed
    // handshake, or QUIT. The outer loop used to have no exit at all: every
    // `break` inside it left only the inner read loop, so a thread went on
    // re-reading a closed socket at full CPU for the life of the process.
    let mut done = false;

    loop {
        let mut guard = match peer.slot.lock() {
            Ok(g) => g,
            Err(_) => break,
        };
        let st = match guard.as_mut() {
            Some(s) => s,
            None => break,
        };
        // Drive the TLS handshake to completion. complete_io reads and writes
        // until the handshake is finished; it flushes ServerHello/Finished as
        // needed and returns once no more TLS progress is required.
        if st.conn.is_handshaking() {
            let mut handshake_done = false;
            while !handshake_done {
                match st.conn.complete_io(&mut st.tcp) {
                    Ok(_) => {
                        handshake_done = !st.conn.is_handshaking();
                        if st.conn.is_handshaking() {
                            continue;
                        }
                    }
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::WouldBlock {
                            continue;
                        }
                        // A plaintext client retrying against the TLS listener
                        // loops this path thousands of times a minute; logging
                        // each attempt once filled tens of megabytes in
                        // minutes. Time-gated, not count-gated: at most one
                        // line per minute no matter how hard the flood runs,
                        // carrying the running total.
                        let n = HANDSHAKE_ERRORS.fetch_add(1, Ordering::Relaxed);
                        let due = {
                            let mut last = LAST_LOG.lock().unwrap_or_else(|p| p.into_inner());
                            let now = SystemTime::now()
                                .duration_since(UNIX_EPOCH)
                                .map(|d| d.as_secs())
                                .unwrap_or(0);
                            if now.saturating_sub(last.1) >= 60 || last.1 == 0 {
                                last.0 = n;
                                last.1 = now;
                                true
                            } else {
                                false
                            }
                        };
                        if due {
                            let peer = st
                                .tcp
                                .peer_addr()
                                .map(|a| a.to_string())
                                .unwrap_or_else(|_| "unknown".into());
                            eprintln!(
                                "chat-server-rs: handshake error #{} so far from {} (one line per minute): {:?}",
                                n + 1,
                                peer,
                                e
                            );
                        }
                        // A handshake that cannot complete never will on this
                        // socket; retrying it is the spin this flag ends.
                        done = true;
                        break;
                    }
                }
            }
        }
        // Deliver anything other connections queued for this one. This thread
        // owns the socket, so this is where a broadcast becomes a write -- a
        // slow peer blocks only itself here. `read_tls` below bounds the wait
        // to its 200ms timeout, so this runs at least that often.
        let (pending, marked_dead) = peer.drain();
        for line in &pending {
            w(st, line);
        }
        if marked_dead || st.closed {
            // Either a broadcaster gave up on this peer, or one of those writes
            // failed. Both mean the connection is finished; the read below is
            // harmless (it is bounded and `w` is now a no-op) and the teardown
            // at the bottom of the loop runs on this pass.
            st.closed = true;
            done = true;
        }
        // After the handshake, catch any further buffered raw bytes and
        // decrypt them before draining plaintext. read_tls returning Ok(0) is
        // the peer's TCP close: the authoritative "this connection is over"
        // signal, and discarding it was how dead sockets stayed in CLOSE-WAIT.
        if let Ok(0) = st.conn.read_tls(&mut st.tcp) {
            done = true;
        }
        let _ = st.conn.process_new_packets();
        while st.conn.write_tls(&mut st.tcp).unwrap_or(0) > 0 {}
        while let Ok(n) = st.conn.reader().read(&mut raw) {
            if n == 0 {
                // Clean TLS EOF.
                done = true;
                break;
            }
            for chunk in raw[..n].split_inclusive(|&b| b == b'\n') {
                let line = std::str::from_utf8(chunk).unwrap_or("");
                let line = line.trim_end_matches(['\r', '\n']);
                if line.is_empty() {
                    continue;
                }
                let msg = match Message::parse(line) {
                    Ok(m) => m,
                    Err(_) => {
                        w(st, "ERROR :malformed line");
                        continue;
                    }
                };
                let verb = msg.command.clone();
                let params = msg.params.clone();
                let trailing = msg.trailing.clone();

                if verb == "CAP" {
                    w(st, "CAP * LS :");
                    continue;
                }
                if verb == "PING" {
                    let tok = params.first().cloned().unwrap_or_default();
                    w(st, &format!("PONG {}", tok));
                    continue;
                }
                if verb == "NICK" {
                    let new_nick = trailing
                        .clone()
                        .or_else(|| params.first().cloned())
                        .unwrap_or_default();
                    if !valid_nick(&new_nick) {
                        w(st, "ERROR :invalid nick");
                        continue;
                    }
                    if sess.registered || !sess.nick.is_empty() {
                        sess.nick = new_nick.clone();
                        st.nick = new_nick.clone();
                        continue;
                    }
                    let in_use = {
                        if let Ok(nicks) = hub.nicks.lock() {
                            nicks.contains_key(&new_nick)
                        } else {
                            false
                        }
                    };
                    if in_use {
                        let n = new_nick.clone();
                        let sn = sess.server_name.clone();
                        w(
                            st,
                            &format!(
                                ":{} {} {} :Nickname is already in use",
                                sn,
                                numerics::ERR_NICKNAMEINUSE,
                                n
                            ),
                        );
                        continue;
                    }
                    sess.nick = new_nick.clone();
                    st.nick = new_nick.clone();
                    if let Ok(mut nicks) = hub.nicks.lock() {
                        nicks.insert(new_nick.clone(), idx as u64);
                    }
                    continue;
                }
                if verb == "USER" {
                    // RFC 1459: USER <username> <mode> <unused> :<realname>, but
                    // clients vary. Use the first parameter as the username and
                    // the fourth (or "localhost") for the host.
                    sess.user = params.first().cloned().unwrap_or_else(|| "*.net".into());
                    sess.host = params
                        .get(3)
                        .cloned()
                        .filter(|h| !h.is_empty())
                        .unwrap_or_else(|| "localhost".into());
                    st.user = sess.user.clone();
                    st.host = sess.host.clone();
                    // Once both NICK and USER are present, registration is
                    // complete: send the welcome block now (not on the next
                    // verb) so the client does not stall waiting for it.
                    if !sess.registered && !sess.nick.is_empty() {
                        sess.registered = true;
                        let sn = sess.server_name.clone();
                        let n = sess.nick.clone();
                        for (code, text) in [
                            (numerics::RPL_WELCOME, "Welcome to the chat server"),
                            (numerics::RPL_YOURHOST, "Your host is the chat server"),
                            (
                                numerics::RPL_CREATED,
                                "This server was created for agent chat",
                            ),
                            (numerics::RPL_MYINFO, "ai-skills chat 1.0"),
                        ] {
                            w(st, &numeric(&sn, code, &n, text).serialize());
                        }
                        w(
                            st,
                            &numeric(
                                &sn,
                                numerics::RPL_ISUPPORT,
                                &n,
                                "NICKLEN=32 CHANNELLEN=32 PREFIX=(o)@ TARGMAX=PRIVMSG:4,NOTICE:4",
                            )
                            .serialize(),
                        );
                        w(
                            st,
                            &numeric(&sn, numerics::RPL_MOTDSTART, &n, "chat server").serialize(),
                        );
                        w(
                            st,
                            &numeric(&sn, numerics::RPL_MOTD, &n, "agent-to-agent chat")
                                .serialize(),
                        );
                        w(
                            st,
                            &numeric(&sn, numerics::RPL_ENDOFMOTD, &n, "end of MOTD").serialize(),
                        );
                    }
                    continue;
                }
                if verb == "QUIT" {
                    // Fall through to the single teardown path at the bottom of
                    // the loop. This used to `return` here, which skipped it
                    // entirely, and three things followed (B127): the socket
                    // was never closed, so the process leaked one descriptor
                    // per QUIT until it hit its limit and stopped accepting
                    // connections -- silently, with every later message lost;
                    // the nick's channel membership was never dropped, leaving
                    // ghosts in NAMES; and the peer's outbox went on accepting
                    // broadcasts for a connection that was gone.
                    //
                    // `chat-client-rs send` sends QUIT as its last act, so this
                    // was one leaked descriptor per message on the bus.
                    w(st, "ERROR :bye");
                    st.closed = true;
                    sess.closed = true;
                    break;
                }

                if !sess.registered && !sess.nick.is_empty() && !sess.user.is_empty() {
                    sess.registered = true;
                    let sn = sess.server_name.clone();
                    let n = sess.nick.clone();
                    for (code, text) in [
                        (numerics::RPL_WELCOME, "Welcome to the chat server"),
                        (numerics::RPL_YOURHOST, "Your host is the chat server"),
                        (
                            numerics::RPL_CREATED,
                            "This server was created for agent chat",
                        ),
                        (numerics::RPL_MYINFO, "ai-skills chat 1.0"),
                    ] {
                        w(st, &numeric(&sn, code, &n, text).serialize());
                    }
                    w(
                        st,
                        &numeric(
                            &sn,
                            numerics::RPL_ISUPPORT,
                            &n,
                            "NICKLEN=32 CHANNELLEN=32 PREFIX=(o)@ TARGMAX=PRIVMSG:4,NOTICE:4",
                        )
                        .serialize(),
                    );
                    w(
                        st,
                        &numeric(&sn, numerics::RPL_MOTDSTART, &n, "chat server").serialize(),
                    );
                    w(
                        st,
                        &numeric(&sn, numerics::RPL_MOTD, &n, "agent-to-agent chat").serialize(),
                    );
                    w(
                        st,
                        &numeric(&sn, numerics::RPL_ENDOFMOTD, &n, "end of MOTD").serialize(),
                    );
                    // Fall through so a JOIN/PRIVMSG issued in the same burst
                    // (as real clients do) is not swallowed by registration.
                }
                if verb == "NICK" || verb == "USER" {
                    continue;
                }

                match verb.as_str() {
                    "JOIN" => {
                        let chan = params.first().cloned().unwrap_or_default();
                        if !valid_chan(&chan) {
                            w(st, "ERROR :invalid channel");
                            continue;
                        }
                        let prefix = format!("{}!{}@{}", sess.nick, sess.user, sess.host);
                        w(st, &format!(":{} JOIN :{}", prefix, chan));
                        if let Ok(mut ch) = hub.channels.lock() {
                            ch.entry(chan.clone()).or_default().push(sess.nick.clone());
                        }
                        if !sess.joined.contains(&chan) {
                            sess.joined.push(chan.clone());
                        }
                        peer.set_joined(&sess.joined);
                        let members = {
                            if let Ok(ch) = hub.channels.lock() {
                                ch.get(&chan).cloned().unwrap_or_default()
                            } else {
                                vec![]
                            }
                        };
                        let sn = sess.server_name.clone();
                        let n = sess.nick.clone();
                        w(
                            st,
                            &format!(":{} 353 {} = {} :{}", sn, n, chan, members.join(" ")),
                        );
                        w(
                            st,
                            &format!(":{} 366 {} {} :End of /NAMES list", sn, n, chan),
                        );
                    }
                    "PART" => {
                        let chan = params.first().cloned().unwrap_or_default();
                        let prefix = format!("{}!{}@{}", sess.nick, sess.user, sess.host);
                        w(st, &format!(":{} PART {} :", prefix, chan));
                        if let Ok(mut ch) = hub.channels.lock() {
                            if let Some(v) = ch.get_mut(&chan) {
                                v.retain(|m| m != &sess.nick);
                            }
                        }
                        sess.joined.retain(|j| j != &chan);
                        // Broadcast membership is read from the outbox, so a
                        // PART has to update it or a parted connection keeps
                        // receiving the channel.
                        peer.set_joined(&sess.joined);
                    }
                    "NAMES" => {
                        let chan = params.first().cloned().unwrap_or_default();
                        let members = {
                            if let Ok(ch) = hub.channels.lock() {
                                ch.get(&chan).cloned().unwrap_or_default()
                            } else {
                                vec![]
                            }
                        };
                        let sn = sess.server_name.clone();
                        let n = sess.nick.clone();
                        w(
                            st,
                            &format!(":{} 353 {} = {} :{}", sn, n, chan, members.join(" ")),
                        );
                        w(
                            st,
                            &format!(":{} 366 {} {} :End of /NAMES list", sn, n, chan),
                        );
                    }
                    "PRIVMSG" | "NOTICE" => {
                        let (chan, text) = match (params.first().cloned(), trailing.clone()) {
                            (Some(c), Some(t)) => (c, t),
                            _ => {
                                w(st, "ERROR :usage: PRIVMSG #chan :text");
                                continue;
                            }
                        };
                        if !valid_chan(&chan) {
                            let sn = sess.server_name.clone();
                            w(st, &format!(":{} ERROR :invalid channel", sn));
                            continue;
                        }
                        if text.is_empty() {
                            w(st, "ERROR :usage: PRIVMSG #chan :text");
                            continue;
                        }
                        match hub.append(&chan, &sess.nick, &text) {
                            Ok(_) => {
                                let m = Message {
                                    prefix: Some(format!(
                                        "{}!{}@{}",
                                        sess.nick, sess.user, sess.host
                                    )),
                                    command: verb.clone(),
                                    params: vec![chan.clone()],
                                    trailing: Some(text.clone()),
                                };
                                let out = m.serialize();
                                w(st, &out);
                                // Broadcast to OTHER connections by queueing on
                                // each peer's outbox. Deliberately not by
                                // writing to their sockets from this thread:
                                // that write blocks for as long as the peer
                                // declines to read, and it happened while
                                // holding `hub.writers` -- the lock the accept
                                // loop needs -- so one idle subscriber wedged
                                // the whole server (B122).
                                //
                                // `hub.writers` is held only to copy the peer
                                // list, and the offers happen after it is
                                // released, so no second lock is ever taken
                                // under it.
                                let peers: Vec<Arc<Peer>> = match hub.writers.lock() {
                                    Ok(writers) => writers
                                        .iter()
                                        .enumerate()
                                        .filter(|(i, _)| *i != idx)
                                        .map(|(_, p)| Arc::clone(p))
                                        .collect(),
                                    Err(_) => Vec::new(),
                                };
                                for p in &peers {
                                    if p.offer(&chan, &out) == Offer::Dropped {
                                        // The peer is not draining its queue.
                                        // It is marked dead, so its own thread
                                        // tears it down; say so, because a
                                        // silently dropped subscriber looks
                                        // like message loss.
                                        eprintln!(
                                            "chat-server-rs: dropping unresponsive peer on {}: over {} bytes of undelivered messages",
                                            chan, OUTBOX_MAX_BYTES
                                        );
                                    }
                                }
                            }
                            Err(e) => w(st, &format!("ERROR :{}", e)),
                        }
                    }
                    "LASTID" => {
                        let chan = params.first().cloned().unwrap_or_default();
                        if !valid_chan(&chan) {
                            w(st, "ERROR :usage: LASTID #chan");
                            continue;
                        }
                        let id = hub.last_id(&chan);
                        // Private numeric 999: `:server 999 <nick> #chan <id>`.
                        let sn = sess.server_name.clone();
                        let n = sess.nick.clone();
                        w(st, &format!(":{} 999 {} {} {}", sn, n, chan, id));
                    }
                    "FETCH" => {
                        let (chan, since) = match (params.first().cloned(), params.get(1).cloned())
                        {
                            (Some(c), Some(s)) => (c, s),
                            _ => {
                                w(st, "ERROR :usage: FETCH #chan <since-id> [mentions]");
                                continue;
                            }
                        };
                        let ok = valid_chan(&chan)
                            && since.chars().all(|c| c.is_ascii_digit())
                            && !since.is_empty();
                        if !ok {
                            w(st, "ERROR :usage: FETCH #chan <since-id> [mentions]");
                            continue;
                        }
                        let since_id: u64 = since.parse().unwrap_or(0);
                        // Optional third param `mentions` filters to rows that
                        // mention the requesting nick (server-side tracking).
                        let only_mentions = params.get(2).map(|s| s.as_str()) == Some("mentions");
                        let rows = if only_mentions {
                            hub.fetch_mentions(&chan, since_id, &sess.nick)
                        } else {
                            hub.fetch(&chan, since_id)
                        };
                        for row in rows {
                            w(st, row.trim_end_matches('\n'));
                        }
                        w(st, &format!("{} {}", FETCH_END, chan));
                    }
                    "WHO" => {
                        // A standard client sends WHO after JOIN and on channel
                        // open; without a 352/315 pair it leaves the nick list
                        // "unknown". The row shape is the RFC minimum.
                        let target = params.first().cloned().unwrap_or_default();
                        let sn = sess.server_name.clone();
                        let me = sess.nick.clone();
                        let mut rows = String::new();
                        if valid_chan(&target) {
                            let members = {
                                if let Ok(ch) = hub.channels.lock() {
                                    ch.get(&target).cloned().unwrap_or_default()
                                } else {
                                    vec![]
                                }
                            };
                            for m in &members {
                                rows.push_str(&format!(
                                    ":{} 352 {} {} {} {} {} {} H :0 {}\n",
                                    sn, me, target, m, m, sn, m, m
                                ));
                            }
                        } else if hub
                            .nicks
                            .lock()
                            .map(|n| n.contains_key(&target))
                            .unwrap_or(false)
                        {
                            rows.push_str(&format!(
                                ":{} 352 {} {} {} {} {} {} H :0 {}\n",
                                sn, me, target, target, target, sn, target, target
                            ));
                        }
                        for row in rows.lines() {
                            w(st, row);
                        }
                        w(
                            st,
                            &format!(":{} 315 {} {} :End of WHO list", sn, me, target),
                        );
                    }
                    "WHOIS" => {
                        let target = params
                            .first()
                            .or(params.get(1))
                            .cloned()
                            .unwrap_or_default();
                        let sn = sess.server_name.clone();
                        let me = sess.nick.clone();
                        let known = hub
                            .nicks
                            .lock()
                            .map(|n| n.contains_key(&target))
                            .unwrap_or(false);
                        if known && !target.is_empty() {
                            w(
                                st,
                                &format!(
                                    ":{} 311 {} {} {} {} * :{}",
                                    sn, me, target, target, target, target
                                ),
                            );
                            w(
                                st,
                                &format!(":{} 312 {} {} {} :ai-chat server", sn, me, target, sn),
                            );
                        } else {
                            w(
                                st,
                                &format!(":{} 401 {} {} :No such nick/channel", sn, me, target),
                            );
                        }
                        w(
                            st,
                            &format!(":{} 318 {} {} :End of /WHOIS list", sn, me, target),
                        );
                    }
                    "MODE" => {
                        // Queries are answered, and the umode set a standard
                        // client sends at connect (Konversation: `MODE <nick>
                        // +i`) is accepted with a confirmation so it does not
                        // read as an error. Unknown flags are still 501.
                        let target = params.first().cloned().unwrap_or_default();
                        let sn = sess.server_name.clone();
                        let me = sess.nick.clone();
                        if params.len() > 1 {
                            let flags = params.get(1).cloned().unwrap_or_default();
                            if target == sess.nick && valid_umode_set(&flags) {
                                let prefix = format!("{}!{}@{}", sess.nick, sess.user, sess.host);
                                w(st, &format!(":{} MODE {} :{}", prefix, me, flags));
                            } else if valid_chan(&target) {
                                // Channel modes need an operator model the bus
                                // does not have; refuse rather than pretend.
                                w(
                                    st,
                                    &format!(
                                        ":{} 482 {} {} :You're not a channel operator",
                                        sn, me, target
                                    ),
                                );
                            } else {
                                w(st, &format!(":{} 501 {} :Unknown MODE flag", sn, me));
                            }
                        } else if valid_chan(&target) {
                            w(st, &format!(":{} 324 {} {} +", sn, me, target));
                        } else if target == sess.nick {
                            w(st, &format!(":{} 221 {} +", sn, me));
                        } else {
                            w(
                                st,
                                &format!(":{} 401 {} {} :No such nick/channel", sn, me, target),
                            );
                        }
                    }
                    "TOPIC" => {
                        let chan = params.first().cloned().unwrap_or_default();
                        let sn = sess.server_name.clone();
                        let me = sess.nick.clone();
                        if !valid_chan(&chan) {
                            w(st, "ERROR :invalid channel");
                            continue;
                        }
                        match &trailing {
                            None => {
                                let t = hub.topics.lock().ok().and_then(|t| t.get(&chan).cloned());
                                match t {
                                    Some(topic) => {
                                        w(st, &format!(":{} 332 {} {} :{}", sn, me, chan, topic))
                                    }
                                    None => w(
                                        st,
                                        &format!(":{} 331 {} {} :No topic is set", sn, me, chan),
                                    ),
                                }
                            }
                            Some(topic) => {
                                if let Ok(mut t) = hub.topics.lock() {
                                    if topic.is_empty() {
                                        t.remove(&chan);
                                    } else {
                                        t.insert(chan.clone(), topic.clone());
                                    }
                                }
                                let prefix = format!("{}!{}@{}", sess.nick, sess.user, sess.host);
                                w(st, &format!(":{} TOPIC {} :{}", prefix, chan, topic));
                            }
                        }
                    }
                    "LIST" => {
                        let sn = sess.server_name.clone();
                        let me = sess.nick.clone();
                        if let Ok(ch) = hub.channels.lock() {
                            for (chan, members) in ch.iter() {
                                let topic = hub
                                    .topics
                                    .lock()
                                    .ok()
                                    .and_then(|t| t.get(chan).cloned())
                                    .unwrap_or_default();
                                w(
                                    st,
                                    &format!(
                                        ":{} 322 {} {} {} :{}",
                                        sn,
                                        me,
                                        chan,
                                        members.len(),
                                        topic
                                    ),
                                );
                            }
                        }
                        w(st, &format!(":{} 323 {} :End of /LIST", sn, me));
                    }
                    "AWAY" => {
                        let sn = sess.server_name.clone();
                        let me = sess.nick.clone();
                        match &trailing {
                            Some(t) if !t.is_empty() => w(
                                st,
                                &format!(":{} 306 {} :You have been marked as being away", sn, me),
                            ),
                            _ => w(
                                st,
                                &format!(
                                    ":{} 305 {} :You are no longer marked as being away",
                                    sn, me
                                ),
                            ),
                        }
                    }
                    "ISON" => {
                        let sn = sess.server_name.clone();
                        let me = sess.nick.clone();
                        let present: Vec<String> = params
                            .iter()
                            .filter(|p| {
                                hub.nicks
                                    .lock()
                                    .map(|n| n.contains_key(*p))
                                    .unwrap_or(false)
                            })
                            .cloned()
                            .collect();
                        w(st, &format!(":{} 303 {} :{}", sn, me, present.join(" ")));
                    }
                    "USERHOST" => {
                        let sn = sess.server_name.clone();
                        let me = sess.nick.clone();
                        let mut rows: Vec<String> = Vec::new();
                        for p in &params {
                            let known =
                                hub.nicks.lock().map(|n| n.contains_key(p)).unwrap_or(false);
                            if known {
                                rows.push(format!("{}=+{}@{}", p, p, p));
                            }
                        }
                        w(st, &format!(":{} 302 {} :{}", sn, me, rows.join(" ")));
                    }
                    _ => {
                        let sn = sess.server_name.clone();
                        w(
                            st,
                            &format!(":{} 421 {} {} :Unknown command", sn, sess.nick, verb),
                        );
                    }
                }
            }
            if st.closed {
                done = true;
                break;
            }
        }

        // Tear down exactly once, and gather what deregistration needs while
        // the borrow of `st` is still alive.
        if !done {
            drop(guard);
            continue;
        }
        st.closed = true;
        let _ = st.tcp.shutdown(std::net::Shutdown::Both);
        let leaving_nick = st.nick.clone();
        let leaving_chans = sess.joined.clone();
        // Drop the ConnState so the socket closes rather than lingering in
        // CLOSE-WAIT, then release the slot guard BEFORE taking any hub lock:
        // holding a slot while acquiring `writers` is the ABBA deadlock the
        // Hub comments warn about.
        *guard = None;
        drop(guard);
        hub.deregister(&leaving_nick, idx, &leaving_chans);
        break;
    }
}
fn announce_loop(
    port: u16,
    name: String,
    host: String,
    interval_secs: u64,
    beacon_port: u16,
    bcast: String,
) {
    let started = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let beacon = format!(
        "{{\"proto\":\"ai-chat/1\",\"name\":\"{}\",\"host\":\"{}\",\"port\":{},\"started\":{}}}",
        name, host, port, started
    );
    let sock = match std::net::UdpSocket::bind((std::net::Ipv4Addr::UNSPECIFIED, 0)) {
        Ok(s) => s,
        Err(_) => return,
    };
    if sock.set_broadcast(true).is_err() {
        return;
    }
    let addr = format!("{}:{}", bcast, beacon_port);
    loop {
        let _ = sock.send_to(beacon.as_bytes(), &addr);
        std::thread::sleep(Duration::from_secs(interval_secs));
    }
}

// The address a listener on `bind` will actually answer on, or None when the
// bind names every interface and so answers the question not at all.
//
// A hostname is resolved rather than trusted. Taking it at face value was a
// defect: "specific" is not "routable", and on the stock Debian/Ubuntu hosts
// shape a hostname maps to 127.0.1.1, so AI_CHAT_BIND=<hostname> announced a
// loopback-only listener under a name and then broadcast it to the whole
// network. Resolving first means the loopback classification below sees what
// the kernel sees.
fn bind_address(bind: &str) -> Option<std::net::IpAddr> {
    let bind = bind.trim();
    if bind.is_empty() {
        return None;
    }
    if let Ok(ip) = bind.parse::<std::net::IpAddr>() {
        return if ip.is_unspecified() { None } else { Some(ip) };
    }
    // A name: ask the resolver what it means on this machine.
    use std::net::ToSocketAddrs;
    (bind, 0u16)
        .to_socket_addrs()
        .ok()?
        .next()
        .map(|sa| sa.ip())
}

// Whether the client can dial this host at all.
//
// chat-client-rs splits a HOST:PORT on a colon and hands the head to TLS SNI
// (`server_host`, src/chat-client-rs/src/main.rs), so a bare IPv6 literal is
// rejected as an invalid DNS name before it is ever connected. Announcing one
// advertises an address nothing can use; announcing an IPv4 address instead
// would advertise one this listener does not answer on. So an IPv6 bind
// announces nothing and says why. B118 tracks the client-side support.
fn host_is_dialable(host: &str) -> bool {
    !matches!(
        host.parse::<std::net::IpAddr>(),
        Ok(std::net::IpAddr::V6(_))
    )
}

// The address a peer should dial.
//
// The bind address answers this whenever it names one interface, and it is the
// only answer that cannot lie: announcing anything else risks publishing an
// address the listener does not answer on. That was the bug -- the route trick
// below ran unconditionally, so the default loopback bind advertised the LAN
// address and no peer, local or remote, could reach the bus (B115).
//
// Only an unspecified bind (0.0.0.0, ::) leaves the question open, because the
// listener really is on every interface and one of them has to be named. std
// has no interface enumeration, so that case uses the classic route trick: a
// UDP connect to a routable address sends no packet, but the kernel picks the
// primary interface's source and local_addr reports it. Falls back to the
// hostname, then to localhost - a name that still tells a peer "this is not an
// address, look elsewhere".
fn announce_host(bind: &str) -> String {
    if let Ok(h) = std::env::var("CHAT_ANNOUNCE_HOST") {
        if !h.trim().is_empty() {
            return h;
        }
    }
    if let Some(ip) = bind_address(bind) {
        return ip.to_string();
    }
    if let Ok(s) = std::net::UdpSocket::bind(("0.0.0.0", 0)) {
        if s.connect("8.8.8.8:80").is_ok() {
            if let Ok(local) = s.local_addr() {
                if !local.ip().is_loopback() {
                    return local.ip().to_string();
                }
            }
        }
    }
    if let Ok(out) = std::process::Command::new("hostname").output() {
        if out.status.success() {
            let h = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !h.is_empty() {
                return h;
            }
        }
    }
    "localhost".to_string()
}

// How far the beacon should travel: exactly as far as the announced address is
// good for. A loopback announce host is meaningless to another machine -- it
// names that machine's own loopback -- so the packet stays here. Anything else
// goes to the broadcast address.
fn announce_bcast(host: &str) -> String {
    if let Ok(b) = std::env::var("CHAT_BCAST") {
        if !b.trim().is_empty() {
            return b;
        }
    }
    bcast_for_host(host)
}

// The env-free half, so a test can pin the rule without touching the
// process environment that every other test shares.
fn bcast_for_host(host: &str) -> String {
    let loopback = host
        .parse::<std::net::IpAddr>()
        .map(|ip| ip.is_loopback())
        .unwrap_or_else(|_| host == "localhost");
    if loopback {
        "127.0.0.1".to_string()
    } else {
        "255.255.255.255".to_string()
    }
}

// Failed TLS handshakes since start, and the second the last one was logged:
// the log-spam gate reads both.
static HANDSHAKE_ERRORS: AtomicU64 = AtomicU64::new(0);
static LAST_LOG: Mutex<(u64, u64)> = Mutex::new((0, 0));

// The same pair for accept() failures, which have their own gate: a descriptor
// limit reached mid-run is a different condition from a bad handshake, and one
// must not silence the other.
static ACCEPT_ERRORS: AtomicU64 = AtomicU64::new(0);
static LAST_ACCEPT_LOG: Mutex<u64> = Mutex::new(0);

fn main() {
    // Same central default as the client: the tsch-ai-skills XDG home.
    let home = std::env::var("AI_CHAT_HOME").unwrap_or_else(|_| {
        let xdg = std::env::var("XDG_CONFIG_HOME")
            .ok()
            .filter(|v| !v.is_empty());
        let home_dir = std::env::var("HOME").unwrap_or_else(|_| ".".into());
        match xdg {
            Some(v) => format!("{}/tsch-ai-skills/chat", v.trim_end_matches('/')),
            None => format!(
                "{}/.config/tsch-ai-skills/chat",
                home_dir.trim_end_matches('/')
            ),
        }
    });
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        println!("chat-server-rs [PORT]");
        println!("  Start the TLS chat server on PORT or its remembered port.");
        return;
    }
    let bind = std::env::var("AI_CHAT_BIND").unwrap_or_else(|_| "127.0.0.1".into());
    let server_name = std::env::var("CHAT_SERVER_NAME").unwrap_or_else(|_| "server".into());

    let home_path = Path::new(&home);
    let chan_dir = home_path.join("channels");
    if let Err(e) = fs::create_dir_all(&chan_dir) {
        eprintln!(
            "chat-server-rs: cannot create {}: {}",
            chan_dir.display(),
            e
        );
        std::process::exit(66);
    }

    let (crt, key) = match ensure_cert(home_path) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("chat-server-rs: {}", e);
            std::process::exit(69);
        }
    };
    let tls_config = match server_config(&crt, &key) {
        Ok(c) => Arc::new(c),
        Err(e) => {
            eprintln!("chat-server-rs: {}", e);
            std::process::exit(69);
        }
    };

    // Port preference, in order: an explicit argv port wins; otherwise the
    // session's last bound port (server.port) is preferred so a restart keeps
    // the address peers already know; only when nothing is recorded - or the
    // recorded port is taken - does the kernel pick an ephemeral one. The
    // file is both the record and the session config: one source of truth.
    let port: u16 = match args.get(1).and_then(|p| p.parse().ok()) {
        Some(p) => p,
        None => {
            let last = fs::read_to_string(home_path.join("server.port"))
                .ok()
                .and_then(|s| s.trim().parse::<u16>().ok())
                .unwrap_or(0);
            if last == 0 {
                0
            } else if std::net::TcpListener::bind((bind.as_str(), last)).is_ok() {
                // Bind succeeded, but the probe socket owns the port: drop it
                // so the real listener can take the same port below.
                last
            } else {
                eprintln!(
                    "chat-server-rs: session port {} is taken; picking an ephemeral port",
                    last
                );
                0
            }
        }
    };
    let listener = match TcpListener::bind((bind.as_str(), port)) {
        Ok(l) => l,
        Err(e) if port == 0 => {
            eprintln!("chat-server-rs: cannot bind {}: {}", bind, e);
            std::process::exit(69);
        }
        Err(e) => {
            // An explicitly requested or session port can be taken between
            // the probe and the bind; fall back to ephemeral rather than die,
            // and say so - the recorded port is rewritten below.
            eprintln!(
                "chat-server-rs: port {} taken ({}); picking an ephemeral port",
                port, e
            );
            match TcpListener::bind((bind.as_str(), 0)) {
                Ok(l) => l,
                Err(e2) => {
                    eprintln!("chat-server-rs: cannot bind {}: {}", bind, e2);
                    std::process::exit(69);
                }
            }
        }
    };
    let actual = listener.local_addr().map(|a| a.port()).unwrap_or(port);
    fs::write(home_path.join("server.port"), format!("{}\n", actual)).expect("write server.port");
    println!("{}", actual);

    let hub = Arc::new(Hub {
        chan_dir,
        highest: Mutex::new(HashMap::new()),
        channels: Mutex::new(HashMap::new()),
        topics: Mutex::new(HashMap::new()),
        nicks: Mutex::new(HashMap::new()),
        writers: Mutex::new(Vec::new()),
    });

    // Announcing is on unless switched off. A server nobody can discover is
    // useless to the agents this bus exists for: they find it by beacon, and a
    // silent one just makes the next agent start a second bus beside it. Only
    // an explicit CHAT_ANNOUNCE=0 suppresses it.
    if std::env::var("CHAT_ANNOUNCE").unwrap_or_else(|_| "1".into()) != "0" {
        let interval: u64 = std::env::var("CHAT_ANNOUNCE_INTERVAL")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(2);
        let beacon_port: u16 = std::env::var("CHAT_BEACON_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(7780);
        let host = announce_host(&bind);
        if host_is_dialable(&host) {
            let bcast = announce_bcast(&host);
            let name = std::env::var("CHAT_NAME").unwrap_or_else(|_| format!("ai-chat/{}", host));
            eprintln!(
                "chat-server-rs: announcing {}:{} every {}s on UDP {} via {}",
                host, actual, interval, beacon_port, bcast
            );
            std::thread::spawn(move || {
                announce_loop(actual, name, host, interval, beacon_port, bcast)
            });
        } else {
            eprintln!(
                "chat-server-rs: not announcing {}:{} — the client cannot dial a bare IPv6 host (B118); pass --server [{}]:{} or set CHAT_ANNOUNCE_HOST",
                host, actual, host, actual
            );
        }
    }

    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(e) => {
                // Discarding this was how descriptor exhaustion presented as
                // "the bus went quiet": accept() fails, the loop spins on it at
                // full speed, every connect is dropped without being serviced,
                // and nothing is written anywhere to say so. Say it, once a
                // minute, and pause -- an error here is a condition to report,
                // not a nuisance to skip.
                let n = ACCEPT_ERRORS.fetch_add(1, Ordering::Relaxed);
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                let due = {
                    let mut last = LAST_ACCEPT_LOG.lock().unwrap_or_else(|p| p.into_inner());
                    if now.saturating_sub(*last) >= 60 || *last == 0 {
                        *last = now;
                        true
                    } else {
                        false
                    }
                };
                if due {
                    eprintln!(
                        "chat-server-rs: cannot accept connections ({} so far, one line per minute): {}",
                        n + 1,
                        e
                    );
                }
                // Without this the loop burns a core retrying an error that
                // will not clear on its own.
                std::thread::sleep(Duration::from_millis(100));
                continue;
            }
        };
        // A short read timeout lets each serve thread release its slot guard
        // frequently instead of holding it across an indefinitely-blocking
        // read_tls (which would stall broadcasters trying to deliver a message
        // to an idle member).
        stream
            .set_read_timeout(Some(Duration::from_millis(200)))
            .ok();
        // And a write timeout, so a peer that stops reading cannot block a
        // write forever. Without one there was no upper bound at all: the
        // blocked thread never learned the peer was gone, so the connection was
        // never torn down and every later broadcast queued behind it.
        stream.set_write_timeout(Some(WRITE_TIMEOUT)).ok();
        let hub = Arc::clone(&hub);
        let tls_config = Arc::clone(&tls_config);
        let mut writers = hub.writers.lock().unwrap();
        let idx = writers.len();
        let peer = Arc::new(Peer::new());
        writers.push(Arc::clone(&peer));
        drop(writers);
        let server_name = server_name.clone();
        std::thread::spawn(move || {
            let conn = match rustls::ServerConnection::new(tls_config) {
                Ok(c) => c,
                Err(_) => return,
            };
            let state = ConnState {
                conn,
                // The accepted stream is MOVED here, not cloned. It was
                // `stream.try_clone().unwrap_or_else(|_| stream.try_clone().unwrap())`,
                // which was two defects in one expression: it duplicated the
                // descriptor for no reason (nothing below needs `stream`), and
                // its fallback unwrapped a second clone that fails for exactly
                // the same reason the first one did. Under a descriptor limit
                // that is `Os { code: 24, TooManyOpenFiles }` in an unwrap --
                // and this crate is built with `panic = "abort"`, so the whole
                // server died, dropping every other live connection with it.
                // Measured: at `ulimit -n 6` the process aborted with that
                // exact message.
                tcp: stream,
                nick: String::new(),
                user: String::new(),
                host: "localhost".into(),
                closed: false,
            };
            if let Ok(mut g) = peer.slot.lock() {
                *g = Some(state);
            }
            serve(peer, hub, idx, server_name);
        });
    }
}

#[cfg(test)]
mod announce_tests {
    use super::{bcast_for_host, bind_address, host_is_dialable};

    // The default bind names one interface, so it is the address to publish.
    // This is the case that was broken: the route trick ignored the bind and
    // advertised the LAN address while the listener answered only on
    // loopback, so no peer could reach the bus (B115).
    #[test]
    fn a_literal_bind_is_the_address_to_announce() {
        assert_eq!(
            bind_address("127.0.0.1").map(|i| i.to_string()).as_deref(),
            Some("127.0.0.1")
        );
        assert_eq!(
            bind_address("192.168.1.106")
                .map(|i| i.to_string())
                .as_deref(),
            Some("192.168.1.106")
        );
        assert_eq!(
            bind_address(" 127.0.0.1 ")
                .map(|i| i.to_string())
                .as_deref(),
            Some("127.0.0.1")
        );
    }

    // An unspecified bind really is every interface, so the bind cannot say
    // which one a peer should dial and discovery has to work it out.
    #[test]
    fn an_unspecified_bind_answers_nothing() {
        assert_eq!(bind_address("0.0.0.0"), None);
        assert_eq!(bind_address("::"), None);
        assert_eq!(bind_address(""), None);
        assert_eq!(bind_address("   "), None);
    }

    // A hostname is NOT taken at face value. This asserts the rule, not a
    // resolver result: whatever the name maps to, the announced host is an
    // address, never the name. Treating the name as "specific" was the defect
    // -- on the stock Debian hosts shape it resolves to 127.0.1.1, so a
    // loopback-only listener was announced by name and broadcast to the LAN.
    #[test]
    fn a_hostname_bind_is_resolved_not_trusted() {
        // localhost is the one name every machine resolves, and it must come
        // back as a loopback ADDRESS, so bcast_for_host then keeps it local.
        let resolved = bind_address("localhost").expect("localhost resolves");
        assert!(resolved.is_loopback(), "localhost resolved to {resolved}");
        assert_eq!(bcast_for_host(&resolved.to_string()), "127.0.0.1");

        // A name that cannot resolve announces nothing rather than itself.
        assert_eq!(bind_address("no-such-host.invalid"), None);
    }

    // A loopback address is only meaningful on this host: broadcasting it
    // would tell every other machine to dial its own loopback.
    #[test]
    fn a_loopback_announce_host_keeps_the_beacon_local() {
        assert_eq!(bcast_for_host("127.0.0.1"), "127.0.0.1");
        assert_eq!(bcast_for_host("127.0.1.1"), "127.0.0.1");
        assert_eq!(bcast_for_host("::1"), "127.0.0.1");
        assert_eq!(bcast_for_host("localhost"), "127.0.0.1");
    }

    #[test]
    fn a_routable_announce_host_broadcasts() {
        assert_eq!(bcast_for_host("192.168.1.106"), "255.255.255.255");
        assert_eq!(bcast_for_host("10.0.0.7"), "255.255.255.255");
    }

    // An IPv6 literal is announceable only once the client can dial it. Until
    // then advertising it would publish an address that fails at TLS SNI, so
    // the beacon is withheld (B118). Everything else is dialable.
    #[test]
    fn a_bare_ipv6_host_is_not_announceable() {
        assert!(!host_is_dialable("::1"));
        assert!(!host_is_dialable("fe80::1"));
        assert!(host_is_dialable("127.0.0.1"));
        assert!(host_is_dialable("192.168.1.106"));
        assert!(host_is_dialable("localhost"));
        assert!(host_is_dialable("some.host.example"));
    }
}

#[cfg(test)]
mod outbox_tests {
    use super::{Offer, Peer, OUTBOX_MAX_BYTES};

    // A broadcaster only ever queues; it must never queue for a connection that
    // is not in the channel, or the peer would receive traffic it never joined.
    #[test]
    fn offer_skips_a_non_member() {
        let p = Peer::new();
        p.set_joined(&["#ops".to_string()]);
        assert_eq!(p.offer("#other", "hello"), Offer::Skipped);
        let (lines, dead) = p.drain();
        assert!(lines.is_empty(), "queued for a channel it never joined");
        assert!(!dead);
    }

    // The membership the broadcaster reads has to follow a PART, which is why
    // it lives in the outbox rather than in the connection state.
    #[test]
    fn offer_queues_for_a_member_in_order() {
        let p = Peer::new();
        p.set_joined(&["#ops".to_string(), "#dev".to_string()]);
        assert_eq!(p.offer("#ops", "first"), Offer::Queued);
        assert_eq!(p.offer("#dev", "second"), Offer::Queued);
        let (lines, dead) = p.drain();
        assert_eq!(lines, vec!["first".to_string(), "second".to_string()]);
        assert!(!dead);
        // Drained once, delivered once: a second drain must not repeat them.
        assert_eq!(p.drain().0, Vec::<String>::new());
    }

    // The queue is what stops a non-reading peer from blocking anyone else, so
    // it is also what would grow without bound. Past the cap the peer is
    // declared dead and its backlog released, rather than the server growing a
    // buffer for a subscriber that is not consuming it (B122).
    #[test]
    fn offer_drops_a_peer_that_exceeds_the_backlog_cap() {
        let p = Peer::new();
        p.set_joined(&["#ops".to_string()]);
        let line = "x".repeat(64 * 1024);
        let mut queued = 0;
        let mut outcome = Offer::Queued;
        for _ in 0..64 {
            outcome = p.offer("#ops", &line);
            if outcome != Offer::Queued {
                break;
            }
            queued += 1;
        }
        assert_eq!(
            outcome,
            Offer::Dropped,
            "the cap never tripped after {} lines of {} bytes",
            queued,
            line.len()
        );
        assert!(
            queued * line.len() <= OUTBOX_MAX_BYTES,
            "queued {} bytes, over the {} byte cap",
            queued * line.len(),
            OUTBOX_MAX_BYTES
        );
        let (lines, dead) = p.drain();
        assert!(dead, "the peer was over the cap but not marked dead");
        assert!(lines.is_empty(), "a dropped peer kept its backlog");
    }

    // Once dead, a peer must stay skipped: it is being torn down by its own
    // thread, and re-queueing for it is how a timed-out peer was retried
    // forever.
    #[test]
    fn a_dead_peer_is_never_queued_for_again() {
        let p = Peer::new();
        p.set_joined(&["#ops".to_string()]);
        p.mark_dead();
        assert_eq!(p.offer("#ops", "hello"), Offer::Skipped);
        let (lines, dead) = p.drain();
        assert!(dead);
        assert!(lines.is_empty());
    }
}
