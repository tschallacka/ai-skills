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

use chat_proto::message::{numeric, numerics, Message, Tag, FETCH_END};
use stale_lock::StaleLock;

// The server's capability registry (T133): every capability this server can
// negotiate via CAP REQ. A generic list, not a hardcoded single flag, so a
// second capability later is one entry here plus its own wiring at the point
// that uses it -- not a second copy of the negotiation state machine.
const CAPABILITIES: &[&str] = &["message-tags"];

// The CAP REQ decision: IRCv3 requires all-or-nothing for a multi-capability
// request (a partial ACK/NAK is not a valid reply), and the echoed list must
// be exactly the requested string, unchanged -- the caller does that echo, so
// this returns only the names, for the caller to record when accepted. Pure
// and unit-tested separately from the connection plumbing around it.
fn negotiate_req(requested: &str) -> (bool, Vec<String>) {
    let names: Vec<&str> = requested.split_whitespace().collect();
    let all_supported = !names.is_empty() && names.iter().all(|n| CAPABILITIES.contains(n));
    (all_supported, names.iter().map(|n| n.to_string()).collect())
}

// How long a socket write may block before the peer is declared unresponsive,
// and how much undelivered broadcast one peer may accumulate. Both are bounds
// on damage a single peer can do, not tuning knobs: without the first, a peer
// that stops reading blocks its own thread forever; without the second, the
// queue that keeps that thread from blocking anyone else grows without bound.
const WRITE_TIMEOUT: Duration = Duration::from_secs(5);
const OUTBOX_MAX_BYTES: usize = 1 << 20;
// B161: an append that dies mid-critical-section (killed process, panic)
// leaves the lock file behind with nobody left to release it. A normal
// append holds it only long enough to scan the log and write one line, so a
// few seconds of staleness margin above worst-case disk latency is generous
// without letting a merely slow append get its lock stolen out from under it.
const CHANNEL_LOCK_STALE_AFTER: Duration = Duration::from_secs(5);

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
    // Mirrors this connection's own Session::caps, so a broadcaster on a
    // DIFFERENT thread can decide what to queue for this peer (e.g. whether
    // to add a message-tags msgid) without touching that connection's Session,
    // which lives on its own thread's stack and is not otherwise reachable.
    caps: Mutex<Vec<String>>,
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
            caps: Mutex::new(Vec::new()),
        }
    }

    /// Record this connection's currently negotiated (ACK'd) capabilities.
    fn set_caps(&self, caps: &[String]) {
        if let Ok(mut c) = self.caps.lock() {
            *c = caps.to_vec();
        }
    }

    /// Whether this connection has negotiated the named capability.
    fn has_cap(&self, name: &str) -> bool {
        self.caps
            .lock()
            .map(|c| c.iter().any(|cap| cap == name))
            .unwrap_or(false)
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
    fn deregister(&self, nick: &str, prefix: &str, idx: usize, chans: &[String]) {
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
        // Tell each channel the nick is gone (B245). A connection that simply
        // ends - which is the normal case here, since `tail --mention-exit`
        // leaves on every mention - sends no PART, so without this the nick
        // stayed in every other client's list until they reconnected.
        //
        // Relayed after the memberships are dropped and outside that lock:
        // `relay` takes `writers`, and taking it while holding `channels` would
        // put two locks in one place, which is the shape B122 came from.
        //
        // The prefix is the full `nick!user@host`, not a bare nick. A first
        // version sent `:nick QUIT` on the reasoning that a deregister can run
        // for a connection that never registered - but a bare-nick prefix is
        // what Konversation rendered as "[quit] connection closed" with nobody
        // named, so the announcement did not say who left, which is the whole
        // point of sending it. The caller passes the prefix it already has, and
        // the empty-nick guard below still covers the unregistered case.
        if !nick.is_empty() {
            let from = if prefix.is_empty() { nick } else { prefix };
            for chan in chans {
                self.relay(chan, &format!(":{} QUIT :connection closed", from), idx);
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
        let lock_path = self.chan_dir.join(format!("{}.lock", chan));
        // B161: the directory-based lock this replaced checked for a `pid`
        // file that nothing in this crate ever wrote, so its only reclaim
        // path never fired and a lock left behind by a killed process blocked
        // the channel forever. StaleLock's reclaim is mtime-based instead —
        // it needs no cooperating writer to have left a marker behind.
        let _lock = StaleLock::acquire(&lock_path, CHANNEL_LOCK_STALE_AFTER)
            .map_err(|_| std::io::Error::other("lock timeout"))?;
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

    /// Relay one line to every member of `chan` except the connection at
    /// `except_idx`, which has already had it written directly.
    ///
    /// Named `relay` rather than `announce`: the announce_* family in this file is
    /// the UDP discovery beacon, a different thing entirely. Extracted so JOIN,
    /// PART and a closing connection reach the channel the way PRIVMSG does,
    /// rather than each growing its own copy of the fanout.
    /// Membership changes were not announced at all before (B244, B245): the
    /// handlers echoed to the acting connection and told nobody else, so a
    /// client already in the channel never saw a nick arrive or leave and its
    /// nick list showed whoever happened to be present when IT joined.
    ///
    /// The lock discipline is the point and is not incidental. `writers` is held
    /// only long enough to copy the peer list, and every `offer` happens after
    /// it is released: writing to a peer's socket while holding `writers` blocks
    /// for as long as that peer declines to read, and because `writers` is the
    /// lock the accept loop needs, one idle subscriber wedged the whole server
    /// (B122). `Peer::offer` skips a non-member, so passing the channel is all
    /// the scoping this needs.
    /// Every peer but one, snapshotted under `writers` alone -- nothing that
    /// can block is done while holding it. Shared by `relay` and
    /// `relay_privmsg` so the two cannot drift on who gets addressed.
    fn peers_except(&self, except_idx: usize) -> Vec<Arc<Peer>> {
        match self.writers.lock() {
            Ok(writers) => writers
                .iter()
                .enumerate()
                .filter(|(i, _)| *i != except_idx)
                .map(|(_, p)| Arc::clone(p))
                .collect(),
            Err(_) => Vec::new(),
        }
    }

    fn relay(&self, chan: &str, line: &str, except_idx: usize) {
        for p in &self.peers_except(except_idx) {
            if p.offer(chan, line) == Offer::Dropped {
                // The peer is not draining its queue. It is marked dead, so its
                // own thread tears it down; say so, because a silently dropped
                // subscriber looks like message loss.
                eprintln!(
                    "chat-server-rs: dropping unresponsive peer on {}: over {} bytes of undelivered messages",
                    chan, OUTBOX_MAX_BYTES
                );
            }
        }
    }

    /// Like `relay`, but for a PRIVMSG/NOTICE broadcast that carries a
    /// message-tags msgid: each peer gets `tagged_line` if it negotiated
    /// message-tags, `untagged_line` (byte-identical to what it would have
    /// received before T134) otherwise -- one offer() per peer either way, so
    /// there is never a second queued line for the same event.
    fn relay_privmsg(&self, chan: &str, tagged_line: &str, untagged_line: &str, except_idx: usize) {
        for p in &self.peers_except(except_idx) {
            let line = if p.has_cap("message-tags") {
                tagged_line
            } else {
                untagged_line
            };
            if p.offer(chan, line) == Offer::Dropped {
                eprintln!(
                    "chat-server-rs: dropping unresponsive peer on {}: over {} bytes of undelivered messages",
                    chan, OUTBOX_MAX_BYTES
                );
            }
        }
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

// A list query asks for the bans, exceptions or invites on a channel; a change
// adds or removes one. What separates them is the MASK, not the sign: `MODE
// #chan b` and `MODE #chan +b` both ask, and only `MODE #chan +b nick!*@*`
// sets. A sign with no mask cannot act on anything, so it can only be a
// question.
//
// The first version of this got it wrong by keying on the sign, and the error it
// was meant to fix came straight back on the next join, because Konversation
// asks with `+b`. The caller must therefore also check that no mask parameter
// follows; this function only judges the letters.
fn list_mode_query(flags: &str) -> bool {
    let letters = flags.trim_start_matches(['+', '-']);
    !letters.is_empty() && letters.chars().all(|ch| matches!(ch, 'b' | 'e' | 'I'))
}

// The two lines that answer one list query: the list itself, which is always
// empty because the bus keeps no bans, exceptions or invites, and the
// end-of-list numeric a client waits for before it stops expecting entries.
// `None` for a letter this server does not answer, so the caller can skip it.
fn empty_list_reply(server: &str, nick: &str, chan: &str, letter: char) -> Option<[String; 1]> {
    let (end_code, end_text) = match letter {
        'b' => (368, "End of channel ban list"),
        'e' => (349, "End of channel exception list"),
        'I' => (347, "End of channel invite list"),
        _ => return None,
    };
    Some([format!(
        ":{} {} {} {} :{}",
        server, end_code, nick, chan, end_text
    )])
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
    // T133: true from CAP LS/REQ until CAP END. While true, registration is
    // held even once NICK+USER are both known -- a real IRCv3 client sends
    // CAP LS before registering and expects 001 withheld until it says END.
    cap_negotiating: bool,
    // Capabilities this connection has negotiated (ACK'd), so the CAP
    // handling below and the message-tags wiring at PRIVMSG time agree on
    // what this connection asked for.
    caps: Vec<String>,
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
        cap_negotiating: false,
        caps: Vec::new(),
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

    // Complete registration (the 001..376 welcome block) exactly once, and
    // only when nothing is still holding it back: nick and user both known,
    // not already registered, and -- T133 -- not in the middle of CAP
    // negotiation. Three call sites need this (after USER, the general
    // fallback for a burst that skipped straight to another command, and CAP
    // END); before this existed the first two each had their own copy of the
    // same welcome block, which is what let them silently disagree about the
    // hold condition when CAP negotiation was added.
    fn maybe_complete_registration(sess: &mut Session, st: &mut ConnState) {
        if sess.registered || sess.nick.is_empty() || sess.user.is_empty() || sess.cap_negotiating {
            return;
        }
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
                    let sn = sess.server_name.clone();
                    let subcommand = params.first().map(|s| s.to_uppercase()).unwrap_or_default();
                    match subcommand.as_str() {
                        "LS" => {
                            // A client sending CAP LS at all is negotiating,
                            // which holds registration until it says END --
                            // even a bare `CAP LS` with no REQ that follows.
                            sess.cap_negotiating = true;
                            w(st, &format!(":{} CAP * LS :{}", sn, CAPABILITIES.join(" ")));
                        }
                        "LIST" => {
                            w(st, &format!(":{} CAP * LIST :{}", sn, sess.caps.join(" ")));
                        }
                        "REQ" => {
                            sess.cap_negotiating = true;
                            let requested = trailing.clone().unwrap_or_default();
                            let (accepted, names) = negotiate_req(&requested);
                            if accepted {
                                for name in names {
                                    if !sess.caps.contains(&name) {
                                        sess.caps.push(name);
                                    }
                                }
                                peer.set_caps(&sess.caps);
                                w(st, &format!(":{} CAP * ACK :{}", sn, requested));
                            } else {
                                w(st, &format!(":{} CAP * NAK :{}", sn, requested));
                            }
                        }
                        "END" => {
                            sess.cap_negotiating = false;
                            maybe_complete_registration(&mut sess, st);
                        }
                        _ => {
                            w(st, &format!(":{} CAP * NAK :", sn));
                        }
                    }
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
                    // verb) so the client does not stall waiting for it --
                    // unless CAP negotiation is still open (T133), in which
                    // case this is a no-op until CAP END says so.
                    maybe_complete_registration(&mut sess, st);
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

                // Fall through so a JOIN/PRIVMSG issued in the same burst (as
                // real clients do) is not swallowed by registration; a no-op
                // if already registered, still holding for CAP, or missing
                // nick/user.
                maybe_complete_registration(&mut sess, st);
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
                        // Tell the channel, which is what puts this nick in
                        // every other client's list (B244). Without it only a
                        // client joining LATER ever learns the nick, from its
                        // own 353 reply, so whether an agent appears depended on
                        // join order. Announced with the standard `JOIN #chan`
                        // form rather than the trailing-colon spelling echoed
                        // above, since this is what other clients parse.
                        hub.relay(&chan, &format!(":{} JOIN {}", prefix, chan), idx);
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
                        // Announced BEFORE set_joined, deliberately: the parting
                        // peer is excluded by index anyway, and narrowing its
                        // own membership first would make the ordering look
                        // load-bearing when it is not.
                        //
                        // Without this a departed nick stayed in every other
                        // client's list for the life of their connection
                        // (B245), which reads worse than the arrival gap: the
                        // list only ever grew.
                        hub.relay(&chan, &format!(":{} PART {}", prefix, chan), idx);
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
                            Ok((id, _)) => {
                                let prefix =
                                    Some(format!("{}!{}@{}", sess.nick, sess.user, sess.host));
                                let untagged = Message {
                                    tags: Vec::new(),
                                    prefix: prefix.clone(),
                                    command: verb.clone(),
                                    params: vec![chan.clone()],
                                    trailing: Some(text.clone()),
                                }
                                .serialize();
                                // T134: the SAME id FETCH would report for this
                                // message, carried inline on the one broadcast
                                // line for a peer that negotiated message-tags
                                // -- not a second queued line, so the earlier
                                // atomicity concern that ruled out a private
                                // numeric alongside PRIVMSG (see the
                                // confirmation note below) does not apply here.
                                let tagged = Message {
                                    tags: vec![Tag {
                                        key: "msgid".to_string(),
                                        value: Some(id.to_string()),
                                    }],
                                    prefix,
                                    command: verb.clone(),
                                    params: vec![chan.clone()],
                                    trailing: Some(text.clone()),
                                }
                                .serialize();
                                // The sender is told NOTHING here, which is the
                                // RFC 1459 flow: a PRIVMSG is relayed to the
                                // other members, and a client renders its own
                                // line locally. Echoing it made every message
                                // appear twice in a standard client's own
                                // window (B249) - Michael's diagnosis from the
                                // Konversation log was exactly that, "once
                                // conquerer printing itself, and once the
                                // channel sending it".
                                //
                                // An unsolicited acknowledgement is no better.
                                // A first attempt pushed `999 <nick> #chan
                                // <id>` here, and a standard client rendered
                                // the numeric verbatim: `[999] mdibbets
                                // #ai-skills 59`. Trading a duplicate line for
                                // a stray one is not a fix.
                                //
                                // So confirmation is SOLICITED instead: our own
                                // client asks `LASTID #chan` after the PRIVMSG
                                // and reads the 999 that already answers it. A
                                // client that does not ask sees nothing extra,
                                // and the id still proves the line was
                                // persisted rather than merely reflected.
                                //
                                // Broadcast to OTHER connections by queueing on
                                // each peer's outbox rather than writing to
                                // their sockets from this thread.
                                // Hub::relay_privmsg owns that fanout (shared
                                // lock discipline with Hub::relay, B122) and
                                // picks tagged vs. untagged per peer.
                                hub.relay_privmsg(&chan, &tagged, &untagged, idx);
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
                            } else if valid_chan(&target)
                                && params.len() == 2
                                && list_mode_query(&flags)
                            {
                                // A QUERY for the ban list, not an attempt to
                                // set one. `params.len() == 2` is the whole
                                // test: flags and no MASK. `MODE #chan b` and
                                // `MODE #chan +b` both ask; only
                                // `MODE #chan +b nick!*@*` sets.
                                //
                                // Measured rather than assumed, after the sign
                                // was tried as the discriminator and the error
                                // came back on the next join: Konversation asks
                                // with `+b`, which the server logged as
                                // `refusing MODE change on #ai-skills from
                                // mdibbets: flags "+b"`.
                                //
                                // The bus keeps no lists, so each queried
                                // letter gets an empty list and its
                                // end-of-list numeric.
                                for letter in flags.trim_start_matches(['+', '-']).chars() {
                                    for line in
                                        empty_list_reply(&sn, &me, &target, letter).iter().flatten()
                                    {
                                        w(st, line);
                                    }
                                }
                            } else if valid_chan(&target) {
                                // A real change. Channel modes need an operator
                                // model the bus does not have; refuse rather
                                // than pretend.
                                //
                                // Logged with the flags, because a refusal that
                                // does not say what it refused is unreadable
                                // from the outside: B248 was fixed once against
                                // a guess at what a client sends after JOIN,
                                // and the error came back.
                                eprintln!(
                                    "chat-server-rs: refusing MODE change on {} from {}: flags {:?}",
                                    target, me, flags
                                );
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
        // The prefix a QUIT is attributed by. Built here because the session
        // still holds user and host, and a bare nick is what left Konversation
        // rendering "[quit] connection closed" with nobody named.
        let leaving_prefix = if leaving_nick.is_empty() {
            String::new()
        } else {
            format!("{}!{}@{}", leaving_nick, sess.user, sess.host)
        };
        // Drop the ConnState so the socket closes rather than lingering in
        // CLOSE-WAIT, then release the slot guard BEFORE taking any hub lock:
        // holding a slot while acquiring `writers` is the ABBA deadlock the
        // Hub comments warn about.
        *guard = None;
        drop(guard);
        hub.deregister(&leaving_nick, &leaving_prefix, idx, &leaving_chans);
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
    if args.iter().any(|arg| arg == "--version") {
        println!("chat-server-rs {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    // B123: a parse failure on argv[1] (a typo'd flag, "--prot 9999") used to
    // read the same as no argument at all, so the server started anyway and
    // silently bound whatever port a probe's own argument mistake happened to
    // leave available. A malformed argument is refused, never treated as one.
    if let Some(raw) = args.get(1) {
        if raw.parse::<u16>().is_err() {
            eprintln!(
                "chat-server-rs: '{}' is not a valid port; usage: chat-server-rs [PORT]",
                raw
            );
            std::process::exit(64);
        }
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
mod mode_query_tests {
    use super::{empty_list_reply, list_mode_query};

    // RFC 1459 4.2.3 / RFC 2812 3.2.3: omitting the MASK is what makes a ban
    // mode a list request, answered with RPL_BANLIST and RPL_ENDOFBANLIST. The
    // sign does not decide it, and the first version of these tests asserted
    // that it did -- so the error they were written for came straight back on
    // the next join, because Konversation asks with `+b`. The server's own
    // refusal log named it: `refusing MODE change on #ai-skills ... flags "+b"`.
    // The spec had this answer before the log did.
    #[test]
    fn a_list_letter_is_a_query_signed_or_not() {
        assert!(list_mode_query("b"), "MODE #chan b asks for the ban list");
        assert!(
            list_mode_query("+b"),
            "and so does MODE #chan +b, which is what Konversation sends"
        );
        assert!(
            list_mode_query("-b"),
            "a sign with no mask cannot act either"
        );
        assert!(list_mode_query("e"), "exception list");
        assert!(list_mode_query("I"), "invite list");
        assert!(list_mode_query("bI"), "several letters at once");
    }

    // The mask is the CALLER's check (`params.len() == 2`); this function judges
    // only the letters. Asserted so nobody reads a `true` here as "no mask was
    // given" and drops the caller's guard.
    #[test]
    fn the_letters_are_judged_but_the_mask_is_not() {
        assert!(
            list_mode_query("+b"),
            "+b nick!*@* must still refuse, and that is params.len(), not this"
        );
        assert!(!list_mode_query(""), "an empty flag string is not a query");
        assert!(!list_mode_query("+"), "a bare sign names no list");
    }

    // A letter this server does not answer must not be silently swallowed as an
    // empty list: it falls through to the refusal instead.
    #[test]
    fn only_the_list_letters_are_answered() {
        assert!(!list_mode_query("k"), "a key is not a list");
        assert!(!list_mode_query("+k"), "nor is a signed key");
        assert!(
            !list_mode_query("bk"),
            "one unknown letter disqualifies the set"
        );
        assert!(empty_list_reply("s", "me", "#c", 'k').is_none());
    }

    // The client waits for the end-of-list numeric before it stops expecting
    // entries, so the reply must carry the right one per letter.
    #[test]
    fn each_list_query_ends_with_its_own_numeric() {
        let ban = empty_list_reply("srv", "me", "#ops", 'b').expect("b is answered");
        assert_eq!(ban[0], ":srv 368 me #ops :End of channel ban list");
        let exc = empty_list_reply("srv", "me", "#ops", 'e').expect("e is answered");
        assert_eq!(exc[0], ":srv 349 me #ops :End of channel exception list");
        let inv = empty_list_reply("srv", "me", "#ops", 'I').expect("I is answered");
        assert_eq!(inv[0], ":srv 347 me #ops :End of channel invite list");
    }
}

#[cfg(test)]
mod membership_relay_tests {
    use super::{Hub, Peer};
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    // The fanout is what puts a nick in another client's list. Before B244 the
    // JOIN handler wrote only to the joining connection, so a client already in
    // the channel was never told and its nick list showed whoever happened to
    // be present when IT joined.
    fn hub_with(peers: Vec<Arc<Peer>>) -> Hub {
        Hub {
            chan_dir: PathBuf::from("/nonexistent"),
            highest: Mutex::new(HashMap::new()),
            channels: Mutex::new(HashMap::new()),
            topics: Mutex::new(HashMap::new()),
            nicks: Mutex::new(HashMap::new()),
            writers: Mutex::new(peers),
        }
    }

    #[test]
    fn announce_reaches_the_other_members_and_not_the_actor() {
        let actor = Arc::new(Peer::new());
        let watcher = Arc::new(Peer::new());
        actor.set_joined(&["#ops".to_string()]);
        watcher.set_joined(&["#ops".to_string()]);
        let hub = hub_with(vec![Arc::clone(&actor), Arc::clone(&watcher)]);

        hub.relay("#ops", ":new!u@h JOIN #ops", 0);

        let (actor_lines, _) = actor.drain();
        assert!(
            actor_lines.is_empty(),
            "the acting connection is excluded by index; it was written to directly"
        );
        let (watcher_lines, _) = watcher.drain();
        assert_eq!(
            watcher_lines,
            vec![":new!u@h JOIN #ops".to_string()],
            "a member already in the channel must be told a nick arrived"
        );
    }

    // Channel scoping comes from Peer::offer, so announcing must not leak a
    // membership change to a connection that is elsewhere.
    #[test]
    fn announce_skips_a_peer_in_another_channel() {
        let elsewhere = Arc::new(Peer::new());
        elsewhere.set_joined(&["#other".to_string()]);
        let hub = hub_with(vec![Arc::new(Peer::new()), Arc::clone(&elsewhere)]);

        hub.relay("#ops", ":new!u@h JOIN #ops", 0);

        let (lines, _) = elsewhere.drain();
        assert!(lines.is_empty(), "a non-member was told about #ops");
    }

    // B245: a connection that simply ends sends no PART, which is the normal
    // case for `tail --mention-exit`, so deregister has to announce the QUIT or
    // the nick stays listed for the life of every other connection.
    #[test]
    fn deregister_announces_a_quit_to_each_channel() {
        let leaving = Arc::new(Peer::new());
        let watcher = Arc::new(Peer::new());
        watcher.set_joined(&["#ops".to_string(), "#dev".to_string()]);
        let hub = hub_with(vec![Arc::clone(&leaving), Arc::clone(&watcher)]);

        hub.deregister(
            "gone",
            "gone!u@h",
            0,
            &["#ops".to_string(), "#dev".to_string()],
        );

        let (lines, _) = watcher.drain();
        assert_eq!(
            lines,
            vec![
                ":gone!u@h QUIT :connection closed".to_string(),
                ":gone!u@h QUIT :connection closed".to_string()
            ],
            "each channel the nick held must hear that it went, ATTRIBUTED: a bare-nick prefix rendered as \"[quit] connection closed\" with nobody named"
        );
    }

    // The JOIN and PART handlers live inside the connection loop, which needs a
    // live TLS peer to drive, so the tests above reach `relay` directly and
    // prove its semantics rather than that those handlers call it. Removing
    // either call therefore broke nothing -- a fix nothing can fail -- so the
    // call sites are pinned here instead.
    //
    // A source assertion is the weaker kind and this says so: it cannot see
    // whether the line reaches the wire, only that the handler asks. The wire
    // itself needs a raw IRC client (the dev shell has no openssl, deliberately),
    // and in practice a standard client's nick list is the check.
    // The needle is never written out in full here, and that is not stylistic.
    // The first version of this test asserted `src.contains("<the exact relay
    // line>")` -- which passed with the call deleted, because include_str! reads
    // the very file holding the test's own assertion literal. The test was its
    // own evidence. Slicing the handler region and looking for a call inside it
    // cannot be satisfied that way: this module sits after every region below.
    fn region<'a>(src: &'a str, from: &str, to: &str) -> &'a str {
        let start = src
            .find(from)
            .unwrap_or_else(|| panic!("region start not found: {}", from));
        let rest = &src[start..];
        let end = rest
            .find(to)
            .unwrap_or_else(|| panic!("region end not found: {}", to));
        &rest[..end]
    }

    #[test]
    fn the_join_and_part_handlers_relay_to_the_channel() {
        let src = include_str!("main.rs");
        let call = "hub.relay(";

        let join = region(src, "\"JOIN\" =>", "\"PART\" =>");
        assert!(
            join.contains(call),
            "the JOIN handler must relay to the channel, or a client already present never sees the nick arrive (B244)"
        );
        let part = region(src, "\"PART\" =>", "\"NAMES\" =>");
        assert!(
            part.contains(call),
            "the PART handler must relay to the channel, or a departed nick stays in every other list (B245)"
        );
        let dereg = region(src, "fn deregister(", "fn scan_highest(");
        assert!(
            dereg.contains("self.relay("),
            "deregister must relay a QUIT: a dropped connection sends no PART (B245)"
        );
    }

    // A deregister can run for a connection that never registered a nick, and
    // relaying ":  QUIT" would be a malformed line on the wire.
    #[test]
    fn deregister_announces_nothing_for_an_unregistered_connection() {
        let watcher = Arc::new(Peer::new());
        watcher.set_joined(&["#ops".to_string()]);
        let hub = hub_with(vec![Arc::new(Peer::new()), Arc::clone(&watcher)]);

        hub.deregister("", "", 0, &["#ops".to_string()]);

        let (lines, _) = watcher.drain();
        assert!(
            lines.is_empty(),
            "announced a QUIT for a nickless connection"
        );
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

// T133 (CAP negotiation) and T134 (message-tags on broadcast PRIVMSG). Like
// membership_relay_tests above, the live parts (LS/REQ/END over a real
// connection, registration actually held) need a socket and are covered by
// chat/tests/test-chat-cap-negotiation.sh; what is pure here is tested here.
#[cfg(test)]
mod cap_negotiation_tests {
    use super::{negotiate_req, Hub, Peer, CAPABILITIES};
    use std::collections::HashMap;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    fn hub_with(peers: Vec<Arc<Peer>>) -> Hub {
        Hub {
            chan_dir: PathBuf::from("/nonexistent"),
            highest: Mutex::new(HashMap::new()),
            channels: Mutex::new(HashMap::new()),
            topics: Mutex::new(HashMap::new()),
            nicks: Mutex::new(HashMap::new()),
            writers: Mutex::new(peers),
        }
    }

    #[test]
    fn a_registered_capability_is_accepted() {
        let (accepted, names) = negotiate_req("message-tags");
        assert!(accepted);
        assert_eq!(names, vec!["message-tags".to_string()]);
    }

    #[test]
    fn an_unregistered_capability_is_refused() {
        let (accepted, _) = negotiate_req("no-such-capability");
        assert!(!accepted);
    }

    #[test]
    fn a_mixed_request_is_all_or_nothing() {
        // IRCv3: a REQ naming several capabilities is one ACK or one NAK for
        // the whole set, never a partial grant -- one unsupported name among
        // several supported ones must refuse all of them.
        let (accepted, _) = negotiate_req("message-tags no-such-capability");
        assert!(
            !accepted,
            "one unsupported capability must NAK the whole request"
        );
    }

    #[test]
    fn an_empty_request_is_refused() {
        let (accepted, names) = negotiate_req("");
        assert!(!accepted);
        assert!(names.is_empty());
    }

    #[test]
    fn every_capability_the_registry_declares_is_individually_acceptable() {
        // A generic registry (T131's design requirement) means this holds for
        // whatever CAPABILITIES lists, not only "message-tags" by name.
        for cap in CAPABILITIES {
            let (accepted, _) = negotiate_req(cap);
            assert!(accepted, "{cap} is in CAPABILITIES but was refused");
        }
    }

    #[test]
    fn a_peer_with_no_negotiated_caps_has_none() {
        let p = Peer::new();
        assert!(!p.has_cap("message-tags"));
    }

    #[test]
    fn set_caps_is_what_has_cap_reads() {
        let p = Peer::new();
        p.set_caps(&["message-tags".to_string()]);
        assert!(p.has_cap("message-tags"));
        assert!(!p.has_cap("no-such-capability"));
    }

    // T134's own contract: a negotiated peer gets the tagged line, a
    // non-negotiated peer in the SAME channel gets the untagged one, from one
    // relay_privmsg call -- never a second queued line for either.
    #[test]
    fn relay_privmsg_splits_by_negotiated_capability() {
        let tagged_peer = Arc::new(Peer::new());
        let plain_peer = Arc::new(Peer::new());
        tagged_peer.set_joined(&["#ops".to_string()]);
        plain_peer.set_joined(&["#ops".to_string()]);
        tagged_peer.set_caps(&["message-tags".to_string()]);
        let hub = hub_with(vec![Arc::clone(&tagged_peer), Arc::clone(&plain_peer)]);

        hub.relay_privmsg(
            "#ops",
            "@msgid=7 :nick!u@h PRIVMSG #ops :hi",
            ":nick!u@h PRIVMSG #ops :hi",
            usize::MAX,
        );

        let (tagged_lines, _) = tagged_peer.drain();
        assert_eq!(
            tagged_lines,
            vec!["@msgid=7 :nick!u@h PRIVMSG #ops :hi".to_string()]
        );
        let (plain_lines, _) = plain_peer.drain();
        assert_eq!(plain_lines, vec![":nick!u@h PRIVMSG #ops :hi".to_string()]);
    }

    #[test]
    fn relay_privmsg_excludes_the_sender_by_index_like_relay() {
        let sender = Arc::new(Peer::new());
        sender.set_joined(&["#ops".to_string()]);
        let hub = hub_with(vec![Arc::clone(&sender)]);

        hub.relay_privmsg("#ops", "@msgid=1 tagged", "untagged", 0);

        let (lines, _) = sender.drain();
        assert!(
            lines.is_empty(),
            "the sender must not receive its own broadcast"
        );
    }
}

#[cfg(test)]
mod append_lock_tests {
    use super::Hub;
    use std::collections::HashMap;
    use std::sync::Mutex;

    fn hub_at(chan_dir: std::path::PathBuf) -> Hub {
        Hub {
            chan_dir,
            highest: Mutex::new(HashMap::new()),
            channels: Mutex::new(HashMap::new()),
            topics: Mutex::new(HashMap::new()),
            nicks: Mutex::new(HashMap::new()),
            writers: Mutex::new(Vec::new()),
        }
    }

    // B161: a lock this crate never populates with a `pid` file (the old
    // directory-based lock's only reclaim condition) must not be able to
    // block a channel forever. The regression is specifically about
    // RECOVERY, not ordinary contention, so this asserts the append after
    // an abandoned lock succeeds at all -- a version that still spun for the
    // old 10-second ceiling before failing would pass a bare timing
    // assertion by accident if the ceiling were ever shortened elsewhere.
    #[test]
    fn an_abandoned_lock_is_reclaimed_not_blocked_forever() {
        let dir = tempfile::tempdir().unwrap();
        let hub = hub_at(dir.path().to_path_buf());
        std::fs::create_dir_all(&hub.chan_dir).unwrap();
        let lock_path = hub.chan_dir.join("#ops.lock");
        std::fs::write(&lock_path, b"stale-token").unwrap();
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(120);
        {
            let file = std::fs::OpenOptions::new()
                .write(true)
                .open(&lock_path)
                .unwrap();
            file.set_modified(old).unwrap();
        }
        let result = hub.append("#ops", "alice", "hello");
        assert!(
            result.is_ok(),
            "append did not recover an abandoned lock: {result:?}"
        );
        let (id, _line) = result.unwrap();
        assert_eq!(id, 1, "the first message in a fresh channel must be id 1");
    }

    // Ordinary append still works with no lock contention at all: two calls
    // in a row see incrementing ids, over the same lock/unlock path the
    // reclaim test above exercises.
    #[test]
    fn appends_with_no_contention_get_incrementing_ids() {
        let dir = tempfile::tempdir().unwrap();
        let hub = hub_at(dir.path().to_path_buf());
        let (first, _) = hub.append("#ops", "alice", "hello").unwrap();
        let (second, _) = hub.append("#ops", "alice", "again").unwrap();
        assert_eq!((first, second), (1, 2));
    }
}
