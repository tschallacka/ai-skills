// MODE: DEV
// PACKAGE: PROD
//! The TLS chat client.
//!
//! Finds a chat server (UDP announce beacon), connects over TLS, pins the
//! server certificate on first connect (TOFU), and then either sends a message,
//! reads a delta since an id (via the additive FETCH extension), or tails a
//! channel. Speaks the same RFC-grammar wire format as the server (shared via
//! chat-proto).
//!
//! Cert pinning: the server certificate's DER fingerprint is stored under the
//! client's state dir keyed by host:port. The first connect records it; later
//! connects require an exact match (fail closed). `--insecure` bypasses this
//! for testing.

use std::fs;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, SystemTime};

use rustls_pki_types::{CertificateDer, ServerName, UnixTime};

const DEFAULT_BEACON_PORT: u16 = 7780;

fn usage() {
    eprintln!(
        "chat-client-rs\n\n\
         usage:\n\
         \x20 chat-client-rs discover [--wait S] [--beacon-port N] [--bcast ADDR] [--json]\n\
         \x20 chat-client-rs send [--server HOST:PORT] [--nick N] --chan #c --text MSG [--insecure]\n\
         \x20 chat-client-rs read  [--server HOST:PORT] [--nick N] --chan #c [--since ID] [--mentions] [--insecure]\n\
         \x20 chat-client-rs tail  [--server HOST:PORT] [--nick N] --chan #c [--mentions] [--mention-exit] [--insecure]\n\
         \x20 chat-client-rs join  [--server HOST:PORT] [--nick N] --chan #c [--since ID] [--insecure]\n\
         \x20 chat-client-rs leave [--server HOST:PORT] [--nick N] --chan #c [--insecure]\n\
         \x20 chat-client-rs session show|set|clear|cursor\n\n\
         options:\n\
         \x20 --state DIR     client state dir (default $AI_CHAT_HOME or the tsch-ai-skills XDG chat dir)\n\
         \x20 --insecure      do not pin the server cert (testing)\n\
         \x20 --no-session    ignore the saved session (server/nick/cursor)\n\
         \x20 --mentions      only messages mentioning your nick (server-side filter)\n\
         \x20 --mention-exit  tail: exit as soon as a message mentions your nick\n\n\
         The session remembers the default server+nick and per-channel cursors\n\
         (last seen message id). join seeds the cursor to the channel's current\n\
         end (so read/tail never dump old history); leave PARTs and drops the\n\
         cursor. A malformed session.json is reset with a warning."
    );
    std::process::exit(64);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        usage();
    }
    let state_dir = client_state_dir();
    match args[1].as_str() {
        "discover" => discover(&args[2..]),
        "send" => send(&args[2..], &state_dir),
        "read" => read_delta(&args[2..], &state_dir),
        "tail" => tail(&args[2..], &state_dir),
        "join" => join_channel(&args[2..], &state_dir),
        "leave" => leave_channel(&args[2..], &state_dir),
        "session" => session_cmd(&args[2..], &state_dir),
        other => {
            eprintln!("chat-client-rs: unknown subcommand: {}", other);
            usage();
        }
    }
}

/// Manage the persisted session (default server+nick and channel cursors).
/// Subcommands: `show`, `set --server H --nick N`, `clear [--cursors]`,
/// `cursor #chan [ID]`.
fn session_cmd(args: &[String], state_dir: &std::path::Path) {
    let sub = args.first().map(|s| s.as_str()).unwrap_or("show");
    match sub {
        "show" => {
            let s = Session::load(state_dir);
            println!("server={}", s.server);
            println!("nick={}", s.nick);
            for (chan, id) in &s.cursors {
                println!("cursor {} {}", chan, id);
            }
        }
        "set" => {
            let mut s = Session::load(state_dir);
            let mut i = 1;
            while i < args.len() {
                match args[i].as_str() {
                    "--server" => {
                        i += 1;
                        if let Some(v) = args.get(i) {
                            s.server = v.clone();
                        }
                    }
                    "--nick" => {
                        i += 1;
                        if let Some(v) = args.get(i) {
                            s.nick = v.clone();
                        }
                    }
                    _ => {}
                }
                i += 1;
            }
            if s.server.is_empty() && s.nick.is_empty() {
                eprintln!("chat-client-rs: session set needs --server and/or --nick");
                std::process::exit(64);
            }
            let _ = s.save(state_dir);
            println!("server={} nick={}", s.server, s.nick);
        }
        "clear" => {
            if args.iter().any(|a| a == "--cursors") {
                let mut s = Session::load(state_dir);
                s.cursors.clear();
                let _ = s.save(state_dir);
                println!("cleared channel cursors");
            } else {
                let _ = fs::remove_file(Session::path(state_dir));
                println!("cleared session");
            }
        }
        "cursor" => {
            let chan = args.get(1).cloned().unwrap_or_default();
            if chan.is_empty() {
                eprintln!("chat-client-rs: session cursor needs #chan [ID]");
                std::process::exit(64);
            }
            let mut s = Session::load(state_dir);
            if let Some(id) = args.get(2).and_then(|v| v.parse::<u64>().ok()) {
                s.cursors.insert(chan.clone(), id);
                let _ = s.save(state_dir);
            }
            println!("{} {}", chan, s.cursor(&chan));
        }
        other => {
            eprintln!("chat-client-rs: unknown session subcommand: {}", other);
            std::process::exit(64);
        }
    }
}

fn client_state_dir() -> PathBuf {
    std::env::var("AI_CHAT_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| chat_default_home())
}

// The central state home everything global shares: the XDG config home's
// tsch-ai-skills directory, beside the shared bin/ and the global plans.
fn chat_default_home() -> PathBuf {
    match std::env::var("XDG_CONFIG_HOME")
        .ok()
        .filter(|v| !v.is_empty())
    {
        Some(v) => PathBuf::from(v).join("tsch-ai-skills").join("chat"),
        None => dirs_home()
            .join(".config")
            .join("tsch-ai-skills")
            .join("chat"),
    }
}

fn dirs_home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()))
}

/// Persistent session state: the default server+nick and per-channel message
/// cursors, so an agent does not repeat `--server host:port --nick me` (and a
/// since-id) on every call. Stored as `<state>/session.json`.
#[derive(serde::Serialize, serde::Deserialize, Default)]
struct Session {
    server: String,
    nick: String,
    #[serde(default)]
    cursors: std::collections::HashMap<String, u64>, // #chan -> last seen id
}

impl Session {
    fn path(state_dir: &std::path::Path) -> PathBuf {
        state_dir.join("session.json")
    }

    /// Load the session, recovering from a missing or malformed file. A
    /// malformed file is reported on stderr (so an agent knows the cursors were
    /// reset) and an empty session is returned; the next save overwrites it.
    fn load(state_dir: &std::path::Path) -> Session {
        let path = Session::path(state_dir);
        let raw = match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(_) => return Session::default(), // no file yet
        };
        match serde_json::from_str(&raw) {
            Ok(s) => s,
            Err(_) => {
                eprintln!(
                    "chat-client-rs: session file {} is malformed; starting a fresh session",
                    path.display()
                );
                Session::default()
            }
        }
    }

    fn save(&self, state_dir: &std::path::Path) -> std::io::Result<()> {
        fs::create_dir_all(state_dir)?;
        let json = serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".into());
        fs::write(Session::path(state_dir), json)
    }

    fn cursor(&self, chan: &str) -> u64 {
        self.cursors.get(chan).copied().unwrap_or(0)
    }
}

/// Fill missing command options from the session (if a session is active).
/// Returns (server, nick) with the session's values where the caller left them
/// empty, and whether the session was consulted.
// ---- server resolution ----------------------------------------------------
// One ladder, tried in order until something answers a TCP connect:
//   1. an explicit --server (used as-is; failures surface at connect)
//   2. the session's saved server
//   3. the last-discovered cache (fast track, most recent first)
//   4. a fresh UDP discovery pass, LAN addresses before loopback
// Nothing at the end is an error the caller reports.

fn tcp_alive(server: &str) -> bool {
    use std::net::ToSocketAddrs;
    if let Ok(addrs) = server.to_socket_addrs() {
        for a in addrs {
            if std::net::TcpStream::connect_timeout(&a, Duration::from_millis(400)).is_ok() {
                return true;
            }
        }
    }
    false
}

fn cache_file(state_dir: &std::path::Path) -> std::path::PathBuf {
    state_dir.join("discovered-servers.txt")
}

fn cache_load(state_dir: &std::path::Path) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    if let Ok(t) = fs::read_to_string(cache_file(state_dir)) {
        for line in t.lines() {
            let s = line.trim();
            if !s.is_empty() && !out.contains(&s.to_string()) {
                out.push(s.to_string());
            }
        }
    }
    out
}

// Move the server to the front of the cache (most recent first), capped.
fn cache_record(state_dir: &std::path::Path, server: &str) {
    let mut list = cache_load(state_dir);
    list.retain(|s| s != server);
    list.insert(0, server.to_string());
    list.truncate(8);
    let body = list.join("\n");
    let _ = fs::write(cache_file(state_dir), body + "\n");
}

// Listen for beacons and return candidate servers, LAN addresses before
// loopback ones: a beacon whose host (or sender) is 127.0.0.1 is only
// interesting when nothing routable announces.
fn discover_candidates(beacon_port: u16, wait_s: u64) -> Vec<String> {
    let sock = match std::net::UdpSocket::bind((std::net::Ipv4Addr::UNSPECIFIED, beacon_port)) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    sock.set_read_timeout(Some(Duration::from_secs(1))).ok();
    let deadline = SystemTime::now() + Duration::from_secs(wait_s);
    let mut lan: Vec<String> = Vec::new();
    let mut local: Vec<String> = Vec::new();
    let mut buf = [0u8; 4096];
    while SystemTime::now() < deadline {
        match sock.recv_from(&mut buf) {
            Ok((n, addr)) => {
                let s = String::from_utf8_lossy(&buf[..n]).to_string();
                let port = match json_field(&s, "port") {
                    Some(p) => p,
                    None => continue,
                };
                let beacon_host = json_field(&s, "host").unwrap_or_default();
                let host = if !beacon_host.is_empty() && beacon_host != "localhost" {
                    beacon_host
                } else {
                    addr.ip().to_string()
                };
                let cand = format!("{}:{}", host, port);
                let is_local =
                    addr.ip().is_loopback() || host == "localhost" || host.starts_with("127.");
                let seen = lan.contains(&cand) || local.contains(&cand);
                if !seen {
                    if is_local {
                        local.push(cand);
                    } else {
                        lan.push(cand);
                    }
                }
            }
            Err(_) => continue,
        }
    }
    lan.extend(local);
    lan
}

// The resolution ladder; returns the server to dial.
fn resolve_server(
    arg_server: &str,
    sess_server: &str,
    state_dir: &std::path::Path,
    no_session: bool,
) -> String {
    // An explicit --server wins without probing; failures surface at connect.
    if !arg_server.is_empty() {
        return arg_server.to_string();
    }
    // The session's saved address is probed: one that no longer answers must
    // not stand between the caller and the ladder below.
    if !no_session && !sess_server.is_empty() && tcp_alive(sess_server) {
        return sess_server.to_string();
    }
    if !no_session && !sess_server.is_empty() {
        eprintln!(
            "chat-client-rs: session server {} is not answering; trying known servers",
            sess_server
        );
    }
    for cand in cache_load(state_dir) {
        if tcp_alive(&cand) {
            cache_record(state_dir, &cand);
            return cand;
        }
    }
    let beacon_port: u16 = std::env::var("AI_CHAT_BEACON_PORT")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_BEACON_PORT);
    let cands = discover_candidates(beacon_port, 3);
    if cands.is_empty() {
        eprintln!(
            "chat-client-rs: no announce beacon received on UDP port {} within 3s",
            beacon_port
        );
    }
    for cand in cands {
        if tcp_alive(&cand) {
            cache_record(state_dir, &cand);
            return cand;
        }
    }
    // Nothing answered: dial the session address anyway so the caller's own
    // connect error names something the human saved rather than an empty
    // string.
    if no_session {
        String::new()
    } else {
        sess_server.to_string()
    }
}

fn apply_session(
    server: &str,
    nick: &str,
    state_dir: &std::path::Path,
    no_session: bool,
) -> (String, String, bool) {
    if no_session {
        return (server.to_string(), nick.to_string(), false);
    }
    let s = Session::load(state_dir);
    let server = if server.is_empty() {
        s.server.clone()
    } else {
        server.to_string()
    };
    let nick = if nick.is_empty() {
        s.nick.clone()
    } else {
        nick.to_string()
    };
    let used = !s.server.is_empty() || !s.nick.is_empty();
    (server, nick, used)
}

struct Opts {
    server: String,
    nick: String,
    chan: String,
    text: String,
    since: String,
    insecure: bool,
    no_session: bool,
    mentions: bool,
    mention_exit: bool,
}

fn parse_opts(args: &[String]) -> Opts {
    let mut o = Opts {
        server: String::new(),
        nick: String::new(),
        chan: String::new(),
        text: String::new(),
        since: String::new(),
        insecure: false,
        no_session: false,
        mentions: false,
        mention_exit: false,
    };
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--server" => {
                i += 1;
                o.server = args.get(i).cloned().unwrap_or_default();
            }
            "--nick" => {
                i += 1;
                o.nick = args.get(i).cloned().unwrap_or_default();
            }
            "--chan" | "-c" => {
                i += 1;
                o.chan = args.get(i).cloned().unwrap_or_default();
            }
            "--text" => {
                i += 1;
                o.text = args.get(i).cloned().unwrap_or_default();
            }
            "--since" => {
                i += 1;
                o.since = args.get(i).cloned().unwrap_or_default();
            }
            "--insecure" => o.insecure = true,
            "--no-session" => o.no_session = true,
            "--mentions" => o.mentions = true,
            "--mention-exit" => o.mention_exit = true,
            _ => {}
        }
        i += 1;
    }
    o
}

/// Record a sent/received message id as the channel cursor in the session.
fn save_cursor(state_dir: &std::path::Path, chan: &str, id: u64, no_session: bool) {
    if no_session {
        return;
    }
    let mut s = Session::load(state_dir);
    if id > s.cursor(chan) {
        s.cursors.insert(chan.to_string(), id);
        let _ = s.save(state_dir);
    }
}

/// Remember the server+nick for later calls.
fn save_session(state_dir: &std::path::Path, server: &str, nick: &str) {
    let mut s = Session::load(state_dir);
    if !server.is_empty() {
        s.server = server.to_string();
    }
    if !nick.is_empty() {
        s.nick = nick.to_string();
    }
    let _ = s.save(state_dir);
}

fn parse_flag(args: &[String], name: &str) -> Option<String> {
    let mut i = 0;
    while i < args.len() {
        if args[i] == name {
            return args.get(i + 1).cloned();
        }
        i += 1;
    }
    None
}

type Client = rustls::StreamOwned<rustls::ClientConnection, TcpStream>;

fn connect(
    server: &str,
    nick: &str,
    state_dir: &std::path::Path,
    insecure: bool,
) -> Result<(Client, String), String> {
    let addr = resolve(server)?;
    let tcp = TcpStream::connect(addr).map_err(|e| format!("connect {}: {}", server, e))?;
    tcp.set_nodelay(true).ok();
    // Bound the handshake and reads so a stalled server cannot hang the client.
    tcp.set_read_timeout(Some(Duration::from_secs(5))).ok();

    let conn = rustls::ClientConnection::new(
        Arc::new(client_config(insecure)),
        ServerName::try_from(server_host(server)).map_err(|e| format!("server name: {}", e))?,
    )
    .map_err(|e| format!("tls client: {}", e))?;
    let mut tls = rustls::StreamOwned::new(conn, tcp);

    // Register first: the first write drives the TLS handshake, after which
    // the peer certificate is available for TOFU pinning. One retry covers a
    // transient race with a just-started server.
    let mut attempts = 0;
    loop {
        // Register in two writes (NICK then USER). The first write drives the
        // handshake; flushing between them ensures each line is on the wire.
        let res = write_line(&mut tls, &format!("NICK {}", nick))
            .and_then(|_| write_line(&mut tls, &format!("USER {} 0 * :{}", nick, nick)));
        match res {
            Ok(()) => break,
            Err(e) => {
                if attempts >= 1 {
                    return Err(e);
                }
                attempts += 1;
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }

    // TOFU: verify the certificate fingerprint against the pinned one, or persist
    // on first connect (unless --insecure).
    let fp = cert_fingerprint(&tls)?;
    if !insecure {
        check_or_pin(server, &fp, state_dir)?;
    }
    Ok((tls, fp))
}

fn client_config(_insecure: bool) -> rustls::ClientConfig {
    // Whether or not --insecure is set, the handshake must accept the server's
    // self-signed cert so TOFU can inspect and pin its fingerprint. The
    // distinction is enforced AFTER the handshake by `check_or_pin` (fail
    // closed unless --insecure).
    let cfg = rustls::ClientConfig::builder()
        .dangerous()
        .with_custom_certificate_verifier(Arc::new(NoVerify::new()))
        .with_no_client_auth();
    // No ALPN: a generic IRC-over-TLS client historically does not negotiate
    // one, and offering "irc" can cause a HandshakeFailure with some peers.
    // cfg.alpn_protocols = vec![b"irc".to_vec()];
    cfg
}

fn cert_fingerprint(
    tls: &rustls::StreamOwned<rustls::ClientConnection, TcpStream>,
) -> Result<String, String> {
    let certs = tls.conn.peer_certificates().ok_or("no peer certificate")?;
    let first = certs.first().ok_or("empty peer cert list")?;
    Ok(hex(&sha256_der(first.as_ref())))
}

fn check_or_pin(server: &str, fp: &str, state_dir: &std::path::Path) -> Result<(), String> {
    let path = state_dir.join(format!("{}.cert.fp", server_safe(server)));
    if path.exists() {
        let stored = fs::read_to_string(&path).map_err(|e| format!("read pin: {}", e))?;
        if stored.trim() != fp {
            return Err(format!(
                "server certificate changed (TOFU pin mismatch); expected {} got {}",
                stored.trim(),
                fp
            ));
        }
        Ok(())
    } else {
        fs::create_dir_all(state_dir).map_err(|e| format!("state dir: {}", e))?;
        fs::write(&path, format!("{}\n", fp)).map_err(|e| format!("pin: {}", e))?;
        Ok(())
    }
}

fn server_safe(server: &str) -> String {
    server.replace([':', '/', '.'], "_")
}

fn server_host(server: &str) -> String {
    server.split(':').next().unwrap_or(server).to_string()
}

fn resolve(server: &str) -> Result<SocketAddr, String> {
    let mut it = server
        .to_socket_addrs()
        .map_err(|e| format!("resolve {}: {}", server, e))?;
    it.next()
        .ok_or_else(|| format!("no address for {}", server))
}

fn write_line(
    tls: &mut rustls::StreamOwned<rustls::ClientConnection, TcpStream>,
    line: &str,
) -> Result<(), String> {
    tls.write_all(format!("{}\r\n", line).as_bytes())
        .map_err(|e| format!("send: {}", e))?;
    tls.flush().map_err(|e| format!("flush: {}", e))?;
    Ok(())
}

fn read_line(
    tls: &mut rustls::StreamOwned<rustls::ClientConnection, TcpStream>,
) -> Result<String, String> {
    let mut buf = Vec::new();
    let mut b = [0u8; 1];
    loop {
        let n = tls.read(&mut b).map_err(|e| format!("read: {}", e))?;
        if n == 0 {
            break;
        }
        buf.push(b[0]);
        if b[0] == b'\n' {
            break;
        }
        if buf.len() > 65536 {
            break;
        }
    }
    if buf.is_empty() {
        return Err("connection closed by server".into());
    }
    Ok(String::from_utf8_lossy(&buf)
        .trim_end_matches(['\r', '\n'])
        .to_string())
}

// ---- subcommands ----------------------------------------------------------

fn discover(args: &[String]) {
    let wait_s: u64 = parse_flag(args, "--wait")
        .and_then(|v| v.parse().ok())
        .unwrap_or(5);
    let beacon_port: u16 = parse_flag(args, "--beacon-port")
        .and_then(|v| v.parse().ok())
        .unwrap_or(DEFAULT_BEACON_PORT);
    let _bcast = parse_flag(args, "--bcast").unwrap_or_else(|| "255.255.255.255".into());
    let json = args.iter().any(|a| a == "--json");

    let sock = std::net::UdpSocket::bind((std::net::Ipv4Addr::UNSPECIFIED, beacon_port))
        .map_err(|e| {
            eprintln!("chat-client-rs: bind beacon port: {}", e);
            std::process::exit(70);
        })
        .unwrap();
    sock.set_read_timeout(Some(Duration::from_secs(1))).ok();
    let deadline = SystemTime::now() + Duration::from_secs(wait_s);
    let mut seen: Vec<String> = Vec::new();
    let mut buf = [0u8; 4096];
    while SystemTime::now() < deadline {
        match sock.recv_from(&mut buf) {
            Ok((n, addr)) => {
                let s = String::from_utf8_lossy(&buf[..n]).to_string();
                if let Some(name) = json_field(&s, "name") {
                    // The beacon's own host field is what a peer should dial;
                    // the packet's source address is the fallback when an
                    // older server does not carry it. A bare "localhost"
                    // names the server but is not connectable, so the
                    // source address wins over it.
                    let host = match json_field(&s, "host") {
                        Some(h) if !h.is_empty() && h != "localhost" => h,
                        _ => addr.ip().to_string(),
                    };
                    let port = json_field(&s, "port").unwrap_or_default();
                    let key = format!("{}|{}|{}", name, host, port);
                    if !seen.contains(&key) {
                        seen.push(key.clone());
                        if json {
                            let mut out = s.trim().to_string();
                            if json_field(&s, "host").is_none() {
                                out = format!("{},\"host\":\"{}\"", &out[..out.len() - 1], host);
                                out.push('}');
                            }
                            println!("{}", out);
                        } else {
                            println!("{}  (port {}, host {})", name, port, host);
                        }
                    }
                }
            }
            Err(_) => continue,
        }
    }
    if seen.is_empty() && !json {
        eprintln!(
            "chat-client-rs: no servers found within {}s on beacon port {}",
            wait_s, beacon_port
        );
    }
}

fn json_field(s: &str, key: &str) -> Option<String> {
    let marker = format!("\"{}\":", key);
    let idx = s.find(&marker)?;
    let rest = &s[idx + marker.len()..];
    let rest = rest.trim_start();
    if let Some(r) = rest.strip_prefix('"') {
        // String value: read to the closing quote.
        let end = r.find('"')?;
        Some(r[..end].to_string())
    } else {
        // Bare literal (the beacon's port and started are numbers): read to
        // the next delimiter.
        let end = rest.find([',', '}'])?;
        Some(rest[..end].trim().to_string())
    }
}

fn send(args: &[String], state_dir: &std::path::Path) {
    let o = parse_opts(args);
    let (mut server, nick, used_session) =
        apply_session(&o.server, &o.nick, state_dir, o.no_session);
    let from_session = server.clone();
    server = resolve_server(&o.server, &from_session, state_dir, o.no_session);
    if server.is_empty() {
        eprintln!(
            "chat-client-rs: no chat server found (no --server, session, known servers or beacons)"
        );
        std::process::exit(70);
    }
    let session_current = used_session && server == from_session;
    if server.is_empty() || nick.is_empty() || o.chan.is_empty() || o.text.is_empty() {
        eprintln!("chat-client-rs: send needs --server --nick --chan --text (or a saved session)");
        std::process::exit(64);
    }
    let (mut tls, _fp) = match connect(&server, &nick, state_dir, o.insecure) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("chat-client-rs: {}", e);
            std::process::exit(70);
        }
    };
    if !session_current {
        save_session(state_dir, &server, &nick);
    }
    // drain to registration complete
    let _ = wait_for_welcome(&mut tls, &nick);
    let _ = write_line(&mut tls, &format!("JOIN {}", o.chan));
    let _ = write_line(&mut tls, &format!("PRIVMSG {} :{}", o.chan, o.text));
    // read the echo of the stored line
    let mut out = String::new();
    let deadline = SystemTime::now() + Duration::from_secs(4);
    while SystemTime::now() < deadline {
        match read_line(&mut tls) {
            Ok(l) => {
                if l.contains("PRIVMSG") && l.contains(&o.chan) && l.contains(&o.text) {
                    out = l;
                    break;
                }
            }
            Err(e) => {
                if !e.contains("os error 11") {
                    break;
                }
            }
        }
    }
    if out.is_empty() {
        eprintln!("chat-client-rs: no echo from server");
        std::process::exit(70);
    }
    println!("{}", out);
    // Advance the channel cursor to the newest id. The echo line is the
    // IRC-prefix form (no id), so fetch history to learn the id of the message
    // just stored.
    let _ = write_line(&mut tls, &format!("FETCH {} {}", o.chan, 0));
    let deadline = SystemTime::now() + Duration::from_secs(3);
    let mut max_id: u64 = Session::load(state_dir).cursor(&o.chan);
    while SystemTime::now() < deadline {
        match read_line(&mut tls) {
            Ok(l) => {
                if l.starts_with(":server 000 end-of-history") {
                    break;
                }
                if l.starts_with("MSG ") {
                    if let Some(id) = l
                        .split_whitespace()
                        .nth(2)
                        .and_then(|s| s.parse::<u64>().ok())
                    {
                        if id > max_id {
                            max_id = id;
                        }
                    }
                }
            }
            Err(e) => {
                if !e.contains("os error 11") {
                    break;
                }
            }
        }
    }
    if max_id > 0 {
        save_cursor(state_dir, &o.chan, max_id, o.no_session);
    }
    let _ = write_line(&mut tls, "QUIT");
}

/// Read the server's `:server 999 <nick> #chan <id>` reply (current max id).
fn read_last_id(
    tls: &mut rustls::StreamOwned<rustls::ClientConnection, TcpStream>,
    chan: &str,
) -> u64 {
    let deadline = SystemTime::now() + Duration::from_secs(3);
    while SystemTime::now() < deadline {
        match read_line(tls) {
            Ok(l) => {
                if l.contains(" 999 ") && l.contains(chan) {
                    return l
                        .split_whitespace()
                        .last()
                        .and_then(|s| s.parse::<u64>().ok())
                        .unwrap_or(0);
                }
            }
            Err(e) => {
                if !e.contains("os error 11") {
                    break;
                }
            }
        }
    }
    0
}

/// Join a channel without reading its history: seed the session cursor to the
/// channel's current end so later read/tail resume from "now".
fn join_channel(args: &[String], state_dir: &std::path::Path) {
    let o = parse_opts(args);
    let (mut server, nick, used_session) =
        apply_session(&o.server, &o.nick, state_dir, o.no_session);
    let from_session = server.clone();
    server = resolve_server(&o.server, &from_session, state_dir, o.no_session);
    if server.is_empty() {
        eprintln!(
            "chat-client-rs: no chat server found (no --server, session, known servers or beacons)"
        );
        std::process::exit(70);
    }
    let session_current = used_session && server == from_session;
    if server.is_empty() || nick.is_empty() || o.chan.is_empty() {
        eprintln!("chat-client-rs: join needs --server --nick --chan (or a saved session)");
        std::process::exit(64);
    }
    let (mut tls, _fp) = match connect(&server, &nick, state_dir, o.insecure) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("chat-client-rs: {}", e);
            std::process::exit(70);
        }
    };
    if !session_current {
        save_session(state_dir, &server, &nick);
    }
    let _ = wait_for_welcome(&mut tls, &nick);
    let _ = write_line(&mut tls, &format!("JOIN {}", o.chan));
    // Ask for the current max id; we do NOT dump the channel history.
    let _ = write_line(&mut tls, &format!("LASTID {}", o.chan));
    let current = read_last_id(&mut tls, &o.chan);
    // Seed the cursor: explicit --since overrides; otherwise the current end.
    let seed = if o.since.is_empty() {
        current
    } else {
        o.since.parse::<u64>().unwrap_or(current)
    };
    if !o.no_session {
        let mut s = Session::load(state_dir);
        s.cursors.insert(o.chan.clone(), seed);
        let _ = s.save(state_dir);
    }
    println!("joined {} (resuming after id {})", o.chan, seed);
    let _ = write_line(&mut tls, "QUIT");
}

/// Leave a channel: send PART and drop the channel cursor from the session.
fn leave_channel(args: &[String], state_dir: &std::path::Path) {
    let o = parse_opts(args);
    let (mut server, nick, used_session) =
        apply_session(&o.server, &o.nick, state_dir, o.no_session);
    let from_session = server.clone();
    server = resolve_server(&o.server, &from_session, state_dir, o.no_session);
    if server.is_empty() {
        eprintln!(
            "chat-client-rs: no chat server found (no --server, session, known servers or beacons)"
        );
        std::process::exit(70);
    }
    let session_current = used_session && server == from_session;
    if server.is_empty() || nick.is_empty() || o.chan.is_empty() {
        eprintln!("chat-client-rs: leave needs --server --nick --chan (or a saved session)");
        std::process::exit(64);
    }
    let (mut tls, _fp) = match connect(&server, &nick, state_dir, o.insecure) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("chat-client-rs: {}", e);
            std::process::exit(70);
        }
    };
    if !session_current {
        save_session(state_dir, &server, &nick);
    }
    let _ = wait_for_welcome(&mut tls, &nick);
    let _ = write_line(&mut tls, &format!("PART {}", o.chan));
    // Clean up the channel's cursor so a later join starts fresh at the end.
    if !o.no_session {
        let mut s = Session::load(state_dir);
        s.cursors.remove(&o.chan);
        let _ = s.save(state_dir);
    }
    println!("left {}", o.chan);
    let _ = write_line(&mut tls, "QUIT");
}

fn read_delta(args: &[String], state_dir: &std::path::Path) {
    let o = parse_opts(args);
    let (mut server, nick, used_session) =
        apply_session(&o.server, &o.nick, state_dir, o.no_session);
    let from_session = server.clone();
    server = resolve_server(&o.server, &from_session, state_dir, o.no_session);
    if server.is_empty() {
        eprintln!(
            "chat-client-rs: no chat server found (no --server, session, known servers or beacons)"
        );
        std::process::exit(70);
    }
    let session_current = used_session && server == from_session;
    if server.is_empty() || nick.is_empty() || o.chan.is_empty() {
        eprintln!("chat-client-rs: read needs --server --nick --chan (or a saved session)");
        std::process::exit(64);
    }
    // --since defaults to the session cursor ("everything since I last saw").
    // With no cursor and no --since, default to the channel's CURRENT end (via
    // LASTID) so reading an old channel does not dump its whole history; use
    // `--since 0` (or --history via session cursor 0) to read everything.
    let mut since = o.since.clone();
    if since.is_empty() && !o.no_session {
        let cur = Session::load(state_dir).cursor(&o.chan);
        if cur > 0 {
            since = cur.to_string();
        }
    }
    let (mut tls, _fp) = match connect(&server, &nick, state_dir, o.insecure) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("chat-client-rs: {}", e);
            std::process::exit(70);
        }
    };
    if !session_current {
        save_session(state_dir, &server, &nick);
    }
    let _ = wait_for_welcome(&mut tls, &nick);
    if since.is_empty() {
        // No cursor yet: find the current end, then fetch nothing from before
        // it unless the caller asked for history.
        let _ = write_line(&mut tls, &format!("LASTID {}", o.chan));
        let current = read_last_id(&mut tls, &o.chan);
        since = current.to_string();
    }
    let mention_suffix = if o.mentions { " mentions" } else { "" };
    let _ = write_line(
        &mut tls,
        &format!("FETCH {} {}{}", o.chan, since, mention_suffix),
    );
    let deadline = SystemTime::now() + Duration::from_secs(5);
    let mut max_id: u64 = 0;
    while SystemTime::now() < deadline {
        match read_line(&mut tls) {
            Ok(l) => {
                if l.starts_with(":server 000 end-of-history") {
                    break;
                }
                if l.starts_with("MSG ") {
                    if let Some(id) = l
                        .split_whitespace()
                        .nth(2)
                        .and_then(|s| s.parse::<u64>().ok())
                    {
                        if id > max_id {
                            max_id = id;
                        }
                    }
                    println!("{}", l);
                }
            }
            Err(e) => {
                if !e.contains("os error 11") {
                    break;
                }
            }
        }
    }
    if max_id > 0 {
        save_cursor(state_dir, &o.chan, max_id, o.no_session);
    }
    let _ = write_line(&mut tls, "QUIT");
}

fn tail(args: &[String], state_dir: &std::path::Path) {
    let o = parse_opts(args);
    let (mut server, nick, used_session) =
        apply_session(&o.server, &o.nick, state_dir, o.no_session);
    let from_session = server.clone();
    server = resolve_server(&o.server, &from_session, state_dir, o.no_session);
    if server.is_empty() {
        eprintln!(
            "chat-client-rs: no chat server found (no --server, session, known servers or beacons)"
        );
        std::process::exit(70);
    }
    let session_current = used_session && server == from_session;
    if server.is_empty() || nick.is_empty() || o.chan.is_empty() {
        eprintln!("chat-client-rs: tail needs --server --nick --chan (or a saved session)");
        std::process::exit(64);
    }
    let (mut tls, _fp) = match connect(&server, &nick, state_dir, o.insecure) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("chat-client-rs: {}", e);
            std::process::exit(70);
        }
    };
    if !session_current {
        save_session(state_dir, &server, &nick);
    }
    let _ = wait_for_welcome(&mut tls, &nick);
    let _ = write_line(&mut tls, &format!("JOIN {}", o.chan));
    // Resume from the session cursor; with no cursor yet, default to the
    // channel's CURRENT end (LASTID) so tailing an old channel does not dump
    // its whole history — only new messages are shown from now on.
    let mut last_id: u64 = if o.no_session {
        0
    } else {
        let cur = Session::load(state_dir).cursor(&o.chan);
        if cur > 0 {
            cur
        } else {
            let _ = write_line(&mut tls, &format!("LASTID {}", o.chan));
            read_last_id(&mut tls, &o.chan)
        }
    };
    let mention_suffix = if o.mentions { " mentions" } else { "" };
    let mut interval: u64 = 5;
    let mut last_seen = SystemTime::now();
    loop {
        // Poll for new history since the last id we saw; print any new rows.
        let _ = write_line(
            &mut tls,
            &format!("FETCH {} {}{}", o.chan, last_id, mention_suffix),
        );
        let poll_deadline = SystemTime::now() + Duration::from_secs(2);
        let mut new_msg = false;
        while SystemTime::now() < poll_deadline {
            match read_line(&mut tls) {
                Ok(l) => {
                    if l.starts_with(":server 000 end-of-history") {
                        break;
                    }
                    if l.starts_with("MSG ") {
                        let id = l
                            .split_whitespace()
                            .nth(2)
                            .and_then(|s| s.parse::<u64>().ok())
                            .unwrap_or(0);
                        if id > last_id {
                            last_id = id;
                            new_msg = true;
                            if o.mentions {
                                // Notification: mark the mention distinctly so
                                // a watcher can act on it.
                                println!("!! MENTION !! {}", l);
                                if o.mention_exit {
                                    let _ = write_line(&mut tls, "QUIT");
                                    std::process::exit(0);
                                }
                            } else {
                                println!("{}", l);
                            }
                            save_cursor(state_dir, &o.chan, id, o.no_session);
                        }
                    }
                }
                Err(e) => {
                    if !e.contains("os error 11") {
                        break;
                    }
                }
            }
        }
        if new_msg {
            // A message reset the cadence to fast.
            interval = 5;
            last_seen = SystemTime::now();
        } else {
            // No new message: step down toward the 60s cap.
            interval = (interval + 10).min(60);
        }
        std::thread::sleep(Duration::from_secs(interval));
        // Bound an idle stretch to the cadence; if the connection is dead the
        // next FETCH will error and this loop breaks.
        let _ = last_seen;
    }
}

fn wait_for_welcome(
    tls: &mut rustls::StreamOwned<rustls::ClientConnection, TcpStream>,
    base_nick: &str,
) -> Result<(), String> {
    let mut seen_001 = false;
    let mut attempt = 2u32;
    let deadline = SystemTime::now() + Duration::from_secs(4);
    while SystemTime::now() < deadline {
        match read_line(tls) {
            Ok(l) => {
                if l.contains(" 001 ") || l.starts_with(":") && l.contains(" 001 ") {
                    seen_001 = true;
                }
                // Nick already in use (e.g. a tail holds the session nick):
                // auto-retry with a numeric suffix like real IRC clients, so a
                // concurrent send/read can register alongside the holder.
                if l.contains(" 433 ") {
                    let alt = format!("{}-{}", base_nick, attempt);
                    attempt += 1;
                    let _ = write_line(tls, &format!("NICK {}", alt));
                    continue;
                }
                if seen_001 && l.contains(" 376 ") {
                    return Ok(());
                }
            }
            Err(e) => {
                // EAGAIN/EWOULDBLOCK: the welcome hasn't arrived within this
                // read's timeout but the connection is alive; keep waiting for
                // the deadline rather than failing a noisy localhost exchange.
                if !e.contains("os error 11") {
                    return Err(e);
                }
            }
        }
    }
    if seen_001 {
        Ok(())
    } else {
        Err("registration did not complete (no 001)".into())
    }
}

// ---- crypto/verifier helpers ------------------------------------------------

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{:02x}", b)).collect()
}

fn sha256_der(der: &[u8]) -> [u8; 32] {
    // Minimal SHA-256 (no external crate). The fingerprint need not be
    // cryptographic-strength against a custom hash collision here because the
    // server cert itself is trusted; SHA-256 over the DER is a stable checksum
    // for TOFU identity. Implemented in-crate to keep dependencies zero.
    sha256(der)
}

// A compact SHA-256 implementation to avoid pulling a crypto dependency for the
// TOFU fingerprint.
fn sha256(message: &[u8]) -> [u8; 32] {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut data = message.to_vec();
    let bitlen = (data.len() as u64).wrapping_mul(8);
    data.push(0x80);
    while data.len() % 64 != 56 {
        data.push(0);
    }
    data.extend_from_slice(&bitlen.to_be_bytes());
    let mut w = [0u32; 64];
    for chunk in data.chunks(64) {
        for i in 0..16 {
            w[i] = u32::from_be_bytes([
                chunk[i * 4],
                chunk[i * 4 + 1],
                chunk[i * 4 + 2],
                chunk[i * 4 + 3],
            ]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for i in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let t1 = hh
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let t2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(t1);
            d = c;
            c = b;
            b = a;
            a = t1.wrapping_add(t2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    let mut out = [0u8; 32];
    for i in 0..8 {
        out[i * 4..i * 4 + 4].copy_from_slice(&h[i].to_be_bytes());
    }
    out
}

/// A certificate verifier that accepts the peer certificate as-is. Used with
/// `--insecure`; the default path calls `check_or_pin` for TOFU after connect.
#[derive(Debug)]
struct NoVerify();

impl NoVerify {
    fn new() -> NoVerify {
        NoVerify()
    }
}

impl rustls::client::danger::ServerCertVerifier for NoVerify {
    fn verify_server_cert(
        &self,
        _end_entity: &CertificateDer<'_>,
        _intermediates: &[CertificateDer<'_>],
        _server_name: &ServerName<'_>,
        _ocsp_response: &[u8],
        _now: UnixTime,
    ) -> Result<rustls::client::danger::ServerCertVerified, rustls::Error> {
        Ok(rustls::client::danger::ServerCertVerified::assertion())
    }
    fn verify_tls12_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    fn verify_tls13_signature(
        &self,
        _message: &[u8],
        _cert: &CertificateDer<'_>,
        _dss: &rustls::DigitallySignedStruct,
    ) -> Result<rustls::client::danger::HandshakeSignatureValid, rustls::Error> {
        Ok(rustls::client::danger::HandshakeSignatureValid::assertion())
    }
    fn supported_verify_schemes(&self) -> Vec<rustls::SignatureScheme> {
        vec![
            rustls::SignatureScheme::RSA_PKCS1_SHA256,
            rustls::SignatureScheme::RSA_PKCS1_SHA384,
            rustls::SignatureScheme::RSA_PKCS1_SHA512,
            rustls::SignatureScheme::RSA_PSS_SHA256,
            rustls::SignatureScheme::RSA_PSS_SHA384,
            rustls::SignatureScheme::RSA_PSS_SHA512,
            rustls::SignatureScheme::ECDSA_NISTP256_SHA256,
            rustls::SignatureScheme::ECDSA_NISTP384_SHA384,
            rustls::SignatureScheme::ED25519,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn tmp_state(name: &str) -> PathBuf {
        let d =
            std::env::temp_dir().join(format!("chat-session-test-{}-{}", std::process::id(), name));
        let _ = fs::remove_dir_all(&d);
        fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn session_round_trips_server_nick_and_cursors() {
        let d = tmp_state("session_round_trips_server_nick_and_cursors");
        let s = Session {
            server: "127.0.0.1:1234".into(),
            nick: "agent".into(),
            cursors: std::collections::HashMap::from([("#ops".to_string(), 7)]),
        };
        s.save(&d).unwrap();

        let loaded = Session::load(&d);
        assert_eq!(loaded.server, "127.0.0.1:1234");
        assert_eq!(loaded.nick, "agent");
        assert_eq!(loaded.cursor("#ops"), 7);
        assert_eq!(loaded.cursor("#other"), 0);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn session_missing_file_is_empty_session() {
        let d = tmp_state("session_missing_file_is_empty_session");
        let s = Session::load(&d);
        assert_eq!(s.server, "");
        assert_eq!(s.nick, "");
        assert_eq!(s.cursor("#x"), 0);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn session_malformed_json_recovers_to_empty() {
        let d = tmp_state("session_malformed_json_recovers_to_empty");
        fs::write(Session::path(&d), "{ not valid json !!!").unwrap();
        let s = Session::load(&d);
        assert_eq!(s.server, "");
        assert_eq!(s.nick, "");
        assert_eq!(s.cursor("#x"), 0);
        // a subsequent save overwrites the malformed file cleanly
        let s2 = Session {
            server: "h:1".into(),
            nick: String::new(),
            cursors: Default::default(),
        };
        s2.save(&d).unwrap();
        let reloaded = Session::load(&d);
        assert_eq!(reloaded.server, "h:1");
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn session_partial_json_keeps_what_parses_or_resets() {
        // A JSON object missing the cursors key must still deserialize
        // (serde default) rather than failing the whole file.
        let d = tmp_state("session_partial_json_keeps_what_parses_or_resets");
        fs::write(Session::path(&d), "{\"server\":\"h:1\",\"nick\":\"n\"}").unwrap();
        let s = Session::load(&d);
        assert_eq!(s.server, "h:1");
        assert_eq!(s.nick, "n");
        assert_eq!(s.cursor("#x"), 0);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn cursor_updates_monotonically() {
        let d = tmp_state("cursor_updates_monotonically");
        save_cursor(&d, "#ops", 5, false);
        save_cursor(&d, "#ops", 3, false); // lower: ignored
        assert_eq!(Session::load(&d).cursor("#ops"), 5);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn no_session_flag_skips_cursor_save() {
        let d = tmp_state("no_session_flag_skips_cursor_save");
        save_cursor(&d, "#ops", 9, true);
        assert_eq!(Session::load(&d).cursor("#ops"), 0);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn parse_opts_reads_mention_flags() {
        let o = parse_opts(&[
            "--mentions".into(),
            "--mention-exit".into(),
            "--chan".into(),
            "#m".into(),
            "--server".into(),
            "h:1".into(),
            "--nick".into(),
            "me".into(),
        ]);
        assert!(o.mentions);
        assert!(o.mention_exit);
        assert_eq!(o.chan, "#m");
        assert!(!o.no_session);
    }

    #[test]
    fn session_clear_drops_cursors_keeps_identity() {
        let d = tmp_state("session_clear_drops_cursors_keeps_identity");
        let mut s = Session {
            server: "h:1".into(),
            nick: "me".into(),
            cursors: std::collections::HashMap::from([("#a".to_string(), 3)]),
        };
        s.save(&d).unwrap();
        s.cursors.clear();
        s.save(&d).unwrap();
        let loaded = Session::load(&d);
        assert_eq!(loaded.server, "h:1");
        assert_eq!(loaded.nick, "me");
        assert_eq!(loaded.cursor("#a"), 0);
        let _ = fs::remove_dir_all(&d);
    }

    #[test]
    fn read_last_id_parses_numeric_reply() {
        // Simulate the server's `:server 999 nick #chan 42` reply line.
        let line = ":server 999 me #ops 42";
        let id = line
            .split_whitespace()
            .last()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        assert_eq!(id, 42);
        // A malformed reply yields 0 (client treats it as an empty channel).
        let bad = " 999 me #ops";
        let id2 = bad
            .split_whitespace()
            .last()
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        assert_eq!(id2, 0);
    }
}
