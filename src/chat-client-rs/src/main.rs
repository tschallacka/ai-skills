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
         \x20 chat-client-rs send --server HOST:PORT --nick N --chan #c --text MSG [--insecure]\n\
         \x20 chat-client-rs read  --server HOST:PORT --nick N --chan #c --since ID [--insecure]\n\
         \x20 chat-client-rs tail  --server HOST:PORT --nick N --chan #c [since-id] [--insecure]\n\n\
         options:\n\
         \x20 --state DIR   client state dir (default $AI_CHAT_HOME or ~/.ai-chat)\n\
         \x20 --insecure    do not pin the server cert (testing)"
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
        other => {
            eprintln!("chat-client-rs: unknown subcommand: {}", other);
            usage();
        }
    }
}

fn client_state_dir() -> PathBuf {
    std::env::var("AI_CHAT_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs_home().join(".ai-chat"))
}

fn dirs_home() -> PathBuf {
    PathBuf::from(std::env::var("HOME").unwrap_or_else(|_| ".".into()))
}

struct Opts {
    server: String,
    nick: String,
    chan: String,
    text: String,
    since: String,
    insecure: bool,
}

fn parse_opts(args: &[String]) -> Opts {
    let mut o = Opts {
        server: String::new(),
        nick: String::new(),
        chan: String::new(),
        text: String::new(),
        since: String::new(),
        insecure: false,
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
            _ => {}
        }
        i += 1;
    }
    o
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

fn connect(server: &str, nick: &str, state_dir: &PathBuf, insecure: bool) -> Result<(Client, String), String> {
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

fn cert_fingerprint(tls: &rustls::StreamOwned<rustls::ClientConnection, TcpStream>) -> Result<String, String> {
    let certs = tls.conn.peer_certificates().ok_or("no peer certificate")?;
    let first = certs.first().ok_or("empty peer cert list")?;
    Ok(hex(&sha256_der(first.as_ref())))
}

fn check_or_pin(server: &str, fp: &str, state_dir: &PathBuf) -> Result<(), String> {
    let path = state_dir.join(format!("{}.cert.fp", server_safe(server)));
    if path.exists() {
        let stored = fs::read_to_string(&path).map_err(|e| format!("read pin: {}", e))?;
        if stored.trim() != fp {
            return Err(format!("server certificate changed (TOFU pin mismatch); expected {} got {}", stored.trim(), fp));
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
    it.next().ok_or_else(|| format!("no address for {}", server))
}

fn write_line(tls: &mut rustls::StreamOwned<rustls::ClientConnection, TcpStream>, line: &str) -> Result<(), String> {
    tls.write_all(format!("{}\r\n", line).as_bytes())
        .map_err(|e| format!("send: {}", e))?;
    tls.flush().map_err(|e| format!("flush: {}", e))?;
    Ok(())
}

fn read_line(tls: &mut rustls::StreamOwned<rustls::ClientConnection, TcpStream>) -> Result<String, String> {
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
    Ok(String::from_utf8_lossy(&buf).trim_end_matches(['\r', '\n']).to_string())
}

// ---- subcommands ----------------------------------------------------------

fn discover(args: &[String]) {
    let wait_s: u64 = parse_flag(args, "--wait").and_then(|v| v.parse().ok()).unwrap_or(5);
    let beacon_port: u16 = parse_flag(args, "--beacon-port").and_then(|v| v.parse().ok()).unwrap_or(DEFAULT_BEACON_PORT);
    let _bcast = parse_flag(args, "--bcast").unwrap_or_else(|| "255.255.255.255".into());
    let json = args.iter().any(|a| a == "--json");

    let sock = std::net::UdpSocket::bind((std::net::Ipv4Addr::UNSPECIFIED, beacon_port))
        .map_err(|e| {
            eprintln!("chat-client-rs: bind beacon port: {}", e);
            std::process::exit(70);
        }).unwrap();
    sock.set_read_timeout(Some(Duration::from_secs(1))).ok();
    let deadline = SystemTime::now() + Duration::from_secs(wait_s);
    let mut seen: Vec<String> = Vec::new();
    let mut buf = [0u8; 4096];
    while SystemTime::now() < deadline {
        match sock.recv_from(&mut buf) {
            Ok((n, _addr)) => {
                let s = String::from_utf8_lossy(&buf[..n]).to_string();
                if let Some(name) = json_field(&s, "name") {
                    let key = format!("{}|{}", name, json_field(&s, "port").unwrap_or_default());
                    if !seen.contains(&key) {
                        seen.push(key.clone());
                        if json {
                            println!("{}", s.trim());
                        } else {
                            println!("{}", key.replace('|', "  (port "));
                            // print with closing paren
                        }
                    }
                }
            }
            Err(_) => continue,
        }
    }
    if seen.is_empty() && !json {
        eprintln!("chat-client-rs: no servers found within {}s on beacon port {}", wait_s, beacon_port);
    }
}

fn json_field(s: &str, key: &str) -> Option<String> {
    let marker = format!("\"{}\":", key);
    let idx = s.find(&marker)?;
    let rest = &s[idx + marker.len()..];
    let rest = rest.trim_start();
    let rest = rest.strip_prefix('"')?;
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn send(args: &[String], state_dir: &PathBuf) {
    let o = parse_opts(args);
    if o.server.is_empty() || o.nick.is_empty() || o.chan.is_empty() || o.text.is_empty() {
        eprintln!("chat-client-rs: send needs --server --nick --chan --text");
        std::process::exit(64);
    }
    let (mut tls, _fp) = match connect(&o.server, &o.nick, state_dir, o.insecure) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("chat-client-rs: {}", e);
            std::process::exit(70);
        }
    };
    // drain to registration complete
    let _ = wait_for_welcome(&mut tls);
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
            Err(_) => break,
        }
    }
    if out.is_empty() {
        eprintln!("chat-client-rs: no echo from server");
        std::process::exit(70);
    }
    println!("{}", out);
    let _ = write_line(&mut tls, "QUIT");
}

fn read_delta(args: &[String], state_dir: &PathBuf) {
    let o = parse_opts(args);
    if o.server.is_empty() || o.nick.is_empty() || o.chan.is_empty() || o.since.is_empty() {
        eprintln!("chat-client-rs: read needs --server --nick --chan --since");
        std::process::exit(64);
    }
    let (mut tls, _fp) = match connect(&o.server, &o.nick, state_dir, o.insecure) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("chat-client-rs: {}", e);
            std::process::exit(70);
        }
    };
    let _ = wait_for_welcome(&mut tls);
    let _ = write_line(&mut tls, &format!("FETCH {} {}", o.chan, o.since));
    let deadline = SystemTime::now() + Duration::from_secs(5);
    while SystemTime::now() < deadline {
        match read_line(&mut tls) {
            Ok(l) => {
                if l.starts_with(":server 000 end-of-history") {
                    break;
                }
                if l.starts_with("MSG ") {
                    println!("{}", l);
                }
            }
            Err(_) => break,
        }
    }
    let _ = write_line(&mut tls, "QUIT");
}

fn tail(args: &[String], state_dir: &PathBuf) {
    let o = parse_opts(args);
    if o.server.is_empty() || o.nick.is_empty() || o.chan.is_empty() {
        eprintln!("chat-client-rs: tail needs --server --nick --chan");
        std::process::exit(64);
    }
    let (mut tls, _fp) = match connect(&o.server, &o.nick, state_dir, o.insecure) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("chat-client-rs: {}", e);
            std::process::exit(70);
        }
    };
    let _ = wait_for_welcome(&mut tls);
    let _ = write_line(&mut tls, &format!("JOIN {}", o.chan));
    // Stream lines until EOF (this is a live tail; Ctrl-C to stop).
    while let Ok(l) = read_line(&mut tls) {
        if l.starts_with(":") && l.contains("PRIVMSG") {
            println!("{}", l);
        }
    }
}

fn wait_for_welcome(tls: &mut rustls::StreamOwned<rustls::ClientConnection, TcpStream>) -> Result<(), String> {
    let mut seen_001 = false;
    let deadline = SystemTime::now() + Duration::from_secs(4);
    while SystemTime::now() < deadline {
        match read_line(tls) {
            Ok(l) => {
                                if l.contains(" 001 ") || l.starts_with(":") && l.contains(" 001 ") {
                    seen_001 = true;
                }
                if seen_001 && l.contains(" 376 ") {
                    return Ok(());
                }
            }
            Err(e) => {
                                return Err(e);
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
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
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
            w[i] = u32::from_be_bytes([chunk[i * 4], chunk[i * 4 + 1], chunk[i * 4 + 2], chunk[i * 4 + 3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) = (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
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
