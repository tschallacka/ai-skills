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
}

fn valid_chan(c: &str) -> bool {
    c.len() > 1
        && c.len() <= 33
        && c.starts_with('#')
        && c[1..]
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
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
                        // complete_io returns when it can't make progress; if we
                        // are no longer handshaking we're done, else loop again.
                        if st.conn.is_handshaking() {
                            // Need more data; complete_io already read+wrote what
                            // it could. Re-loop to process any newly available
                            // plaintext/staged writes.
                            continue;
                        }
                    }
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::WouldBlock {
                            continue;
                        }
                        eprintln!("chat-server-rs: handshake error: {:?}", e);
                        break;
                    }
                }
            }
        } else {
            // Already past handshake: read raw bytes, process, flush.
            let _ = st.conn.read_tls(&mut st.tcp);
            let _ = st.conn.process_new_packets();
            while st.conn.write_tls(&mut st.tcp).unwrap_or(0) > 0 {}
        }
        while let Ok(n) = st.conn.reader().read(&mut raw) {
            if n == 0 {
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
                    "FETCH" => {
                        let (chan, since) = match (params.first().cloned(), params.get(1).cloned())
                        {
                            (Some(c), Some(s)) => (c, s),
                            _ => {
                                w(st, "ERROR :usage: FETCH #chan <since-id>");
                                continue;
                            }
                        };
                        let ok = valid_chan(&chan)
                            && since.chars().all(|c| c.is_ascii_digit())
                            && !since.is_empty();
                        if !ok {
                            w(st, "ERROR :usage: FETCH #chan <since-id>");
                            continue;
                        }
                        let since_id: u64 = since.parse().unwrap_or(0);
                        for row in hub.fetch(&chan, since_id) {
                            w(st, row.trim_end_matches('\n'));
                        }
                        w(st, &format!("{} {}", FETCH_END, chan));
                    }
                    _ => {
                        w(st, "ERROR :unknown command");
                    }
                }
            }
            if st.closed {
                break;
            }
        }
        drop(guard);
    }
}
fn announce_loop(port: u16, name: String, interval_secs: u64, beacon_port: u16, bcast: String) {
    let started = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let beacon = format!(
        "{{\"proto\":\"ai-chat/1\",\"name\":\"{}\",\"port\":{},\"started\":{}}}",
        name, port, started
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

fn main() {
    let home = std::env::var("AI_CHAT_HOME").unwrap_or_else(|_| {
        eprintln!("chat-server-rs: AI_CHAT_HOME must be set");
        std::process::exit(64);
    });
    let args: Vec<String> = std::env::args().collect();
    let port: u16 = args.get(1).and_then(|p| p.parse().ok()).unwrap_or(0);
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

    let listener = match TcpListener::bind((bind.as_str(), port)) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("chat-server-rs: cannot bind {}: {}", bind, e);
            std::process::exit(69);
        }
    };
    let actual = listener.local_addr().map(|a| a.port()).unwrap_or(port);
    fs::write(home_path.join("server.port"), format!("{}\n", actual)).expect("write server.port");
    println!("{}", actual);

    let hub = Arc::new(Hub {
        chan_dir,
        highest: Mutex::new(HashMap::new()),
        channels: Mutex::new(HashMap::new()),
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
        let name =
            std::env::var("CHAT_NAME").unwrap_or_else(|_| format!("ai-chat/{}", "localhost"));
        std::thread::spawn(move || announce_loop(actual, name, interval, beacon_port, bcast));
    }

    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(_) => continue,
        };
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
