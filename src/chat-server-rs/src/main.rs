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

use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chat_proto::message::{numeric, numerics, Message, FETCH_END};

struct ConnState {
    conn: rustls::ServerConnection,
    tcp: TcpStream,
    nick: String,
    user: String,
    host: String,
    joined: Vec<String>,
    closed: bool,
}

impl ConnState {
    fn write_line(&mut self, s: &str) {
        let _ = self
            .conn
            .writer()
            .write_all(format!("{}\r\n", s).as_bytes());
        // Write-only flush: complete_io would block reading for more input,
        // which deadlocks when the peer is also blocked reading a response.
        while self.conn.write_tls(&mut self.tcp).unwrap_or(0) > 0 {}
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
    // zero-sized connection writers: one per live connection, keyed by index
    writers: Mutex<Vec<Arc<Mutex<Option<ConnState>>>>>,
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
    /// again — it got ERR_NICKNAMEINUSE against a connection that no longer
    /// existed, which is why only one client at a time could be connected.
    /// The nick is only surrendered when the registry still points at THIS
    /// connection index; a later connection that took the nick over keeps it.
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
        // The slot itself stays in `writers` so live connections keep their
        // index (broadcast addresses members by index). It now holds None, so
        // it is a few bytes rather than a socket, and the broadcast path
        // already skips an empty slot.
        if let Ok(writers) = self.writers.lock() {
            if let Some(slot) = writers.get(idx) {
                if let Ok(mut g) = slot.lock() {
                    *g = None;
                }
            }
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

fn serve(slot: Arc<Mutex<Option<ConnState>>>, hub: Arc<Hub>, idx: usize, server_name: String) {
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
    // slot guard, so it never re-locks the same slot (no deadlock).
    fn w(st: &mut ConnState, s: &str) {
        if !st.closed {
            st.write_line(s);
        }
    }

    // Set the moment this connection is finished — a dead peer, a failed
    // handshake, or QUIT. The outer loop used to have no exit at all: every
    // `break` inside it left only the inner read loop, so a thread went on
    // re-reading a closed socket at full CPU for the life of the process. One
    // aborted client was measured holding a core for nine minutes, and enough
    // of them starve `accept` until the listener stops answering entirely.
    let mut done = false;

    loop {
        let mut guard = match slot.lock() {
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
                    w(st, "ERROR :bye");
                    st.closed = true;
                    sess.closed = true;
                    if let Ok(mut nicks) = hub.nicks.lock() {
                        nicks.retain(|_, v| *v != idx as u64);
                    }
                    drop(guard);
                    return;
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
                        st.joined = sess.joined.clone();
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
                                // broadcast to OTHER connections (their slots are
                                // different mutexes, so locking them here is safe).
                                if let Ok(writers) = hub.writers.lock() {
                                    for (i, s) in writers.iter().enumerate() {
                                        if i == idx {
                                            continue;
                                        }
                                        if let Ok(mut g) = s.lock() {
                                            if let Some(c) = g.as_mut() {
                                                if !c.closed && c.joined.iter().any(|j| j == &chan)
                                                {
                                                    c.write_line(&out);
                                                }
                                            }
                                        }
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
        let leaving_chans = st.joined.clone();
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

// The address a LAN peer should dial. std has no interface enumeration, so
// this uses the classic route trick: a UDP connect to a routable address
// sends no packet, but the kernel picks the primary interface's source, and
// local_addr reports it. Falls back to the hostname, then to localhost - a
// name that still tells a LAN peer "this is not an address, look elsewhere".
fn announce_host() -> String {
    if let Ok(h) = std::env::var("CHAT_ANNOUNCE_HOST") {
        if !h.trim().is_empty() {
            return h;
        }
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

// Failed TLS handshakes since start, and the second the last one was logged:
// the log-spam gate reads both.
static HANDSHAKE_ERRORS: AtomicU64 = AtomicU64::new(0);
static LAST_LOG: Mutex<(u64, u64)> = Mutex::new((0, 0));

const USAGE: &str = "\
chat-server-rs — the chat message bus server (IRC-shaped, TLS)

usage:
  chat-server-rs [PORT]
  chat-server-rs --help

  PORT  bind this port. Omitted, the port recorded in <home>/server.port is
        reused so a restart keeps the address peers already know; if that one
        is taken, the kernel picks an ephemeral port. The chosen port is
        printed on stdout and written back to <home>/server.port.

environment:
  AI_CHAT_HOME          state home (default $XDG_CONFIG_HOME/tsch-ai-skills/chat)
  AI_CHAT_BIND          bind address (default 127.0.0.1)
  CHAT_SERVER_NAME      name used in numeric replies (default \"server\")
  CHAT_ANNOUNCE=1       broadcast a UDP discovery beacon
  CHAT_ANNOUNCE_HOST    address to advertise (default: the primary interface)
  CHAT_ANNOUNCE_INTERVAL  beacon interval in seconds (default 2)
  CHAT_BEACON_PORT      beacon UDP port (default 7780)
  CHAT_BCAST            broadcast address (default 255.255.255.255)
  CHAT_NAME             advertised server name (default ai-chat/<host>)

exit codes:
  0 help; 64 bad invocation; 66 cannot create the channel directory;
  69 no usable certificate or no bindable address
";

/// What the command line asks for, decided before anything binds.
#[derive(Debug, PartialEq, Eq)]
enum ArgAction {
    /// Print usage, exit 0.
    Help,
    /// Serve. `Some(port)` for an explicit port, `None` to resolve one.
    Serve(Option<u16>),
    /// Refuse with this message, exit 64.
    Reject(String),
}

/// Classify argv WITHOUT touching the filesystem or a socket.
///
/// The server had no `--help` at all: argv[1] was parsed straight as a port,
/// so `--help` failed to parse, yielded None, fell through to the recorded
/// session port, and stood up a REAL server. A `chat-server-rs --help | head`
/// probe therefore left a live bus on the shared port that refused everyone
/// else. Deciding this up front, as pure data, is what keeps help and serve
/// from ever being the same code path again.
fn classify_args(argv: &[String]) -> ArgAction {
    for a in argv {
        if a == "--help" || a == "-h" {
            return ArgAction::Help;
        }
    }
    match argv.len() {
        0 => ArgAction::Serve(None),
        1 => match argv[0].parse::<u16>() {
            Ok(p) => ArgAction::Serve(Some(p)),
            // A typo must not be read as "no port given".
            Err(_) => ArgAction::Reject(format!("not a port: {}", argv[0])),
        },
        _ => ArgAction::Reject(format!("unexpected argument: {}", argv[1])),
    }
}

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let explicit_port = match classify_args(&argv) {
        ArgAction::Help => {
            print!("{}", USAGE);
            std::process::exit(0);
        }
        ArgAction::Reject(message) => {
            eprintln!("chat-server-rs: {}", message);
            eprintln!("chat-server-rs: run with --help for usage");
            std::process::exit(64);
        }
        ArgAction::Serve(port) => port,
    };

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
    let port: u16 = match explicit_port {
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

    if std::env::var("CHAT_ANNOUNCE").unwrap_or_else(|_| "0".into()) == "1" {
        let interval: u64 = std::env::var("CHAT_ANNOUNCE_INTERVAL")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(2);
        let beacon_port: u16 = std::env::var("CHAT_BEACON_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(7780);
        let bcast = std::env::var("CHAT_BCAST").unwrap_or_else(|_| "255.255.255.255".into());
        let host = announce_host();
        let name = std::env::var("CHAT_NAME").unwrap_or_else(|_| format!("ai-chat/{}", host));
        std::thread::spawn(move || announce_loop(actual, name, host, interval, beacon_port, bcast));
    }

    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(_) => continue,
        };
        // A short read timeout lets each serve thread release its slot guard
        // frequently instead of holding it across an indefinitely-blocking
        // read_tls (which would stall broadcasters trying to deliver a message
        // to an idle member).
        stream
            .set_read_timeout(Some(Duration::from_millis(200)))
            .ok();
        let hub = Arc::clone(&hub);
        let tls_config = Arc::clone(&tls_config);
        let mut writers = hub.writers.lock().unwrap();
        let idx = writers.len();
        let slot = Arc::new(Mutex::new(None));
        writers.push(Arc::clone(&slot));
        drop(writers);
        let server_name = server_name.clone();
        std::thread::spawn(move || {
            let conn = match rustls::ServerConnection::new(tls_config) {
                Ok(c) => c,
                Err(_) => return,
            };
            let state = ConnState {
                conn,
                tcp: stream.try_clone().unwrap_or_else(|_| {
                    // Unreachable in practice on a live socket; if it fails the
                    // connection is unusable. Provide the original stream so the
                    // rest of the setup still compiles; serve will drop it on EOF.
                    stream.try_clone().unwrap()
                }),
                nick: String::new(),
                user: String::new(),
                host: "localhost".into(),
                joined: Vec::new(),
                closed: false,
            };
            if let Ok(mut g) = slot.lock() {
                *g = Some(state);
            }
            serve(slot, hub, idx, server_name);
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn help_is_recognised_and_never_becomes_a_serve() {
        // The defect: --help was not a flag at all, so it fell through to the
        // port resolution and started a real server on the shared port.
        assert_eq!(classify_args(&argv(&["--help"])), ArgAction::Help);
        assert_eq!(classify_args(&argv(&["-h"])), ArgAction::Help);
        // Help wins wherever it appears, and never resolves a port.
        assert_eq!(classify_args(&argv(&["1234", "--help"])), ArgAction::Help);
        for a in [
            argv(&["--help"]),
            argv(&["-h"]),
            argv(&["1234", "--help"]),
            argv(&["--help", "extra"]),
        ] {
            assert!(
                !matches!(classify_args(&a), ArgAction::Serve(_)),
                "{:?} must not serve",
                a
            );
        }
    }

    #[test]
    fn no_argument_serves_with_an_unresolved_port() {
        assert_eq!(classify_args(&[]), ArgAction::Serve(None));
    }

    #[test]
    fn a_bare_port_serves_on_exactly_that_port() {
        assert_eq!(
            classify_args(&argv(&["7717"])),
            ArgAction::Serve(Some(7717))
        );
        assert_eq!(classify_args(&argv(&["0"])), ArgAction::Serve(Some(0)));
        assert_eq!(
            classify_args(&argv(&["65535"])),
            ArgAction::Serve(Some(65535))
        );
    }

    #[test]
    fn a_bad_argument_is_refused_not_read_as_no_port() {
        // Silently treating a typo as "no port given" is how --help booted a
        // server; every unparseable first argument is now a refusal.
        for bad in ["--porrt", "abc", "-1", "65536", "70000", "12.5", ""] {
            match classify_args(&argv(&[bad])) {
                ArgAction::Reject(m) => assert!(
                    m.contains("not a port"),
                    "{} rejected with the wrong message: {}",
                    bad,
                    m
                ),
                other => panic!("{} was not refused: {:?}", bad, other),
            }
        }
    }

    #[test]
    fn a_second_argument_is_refused() {
        match classify_args(&argv(&["7717", "extra"])) {
            ArgAction::Reject(m) => assert!(m.contains("unexpected argument"), "message: {}", m),
            other => panic!("expected a refusal, got {:?}", other),
        }
    }
}
