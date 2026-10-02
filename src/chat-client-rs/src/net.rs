// MODE: DEV
// PACKAGE: PROD
//! TLS connect, the TOFU cert pin, line I/O, and the crypto helpers that back
//! it (T101 split out of lib.rs).

use rustls_pki_types::{CertificateDer, ServerName, UnixTime};
use std::fs;
use std::io::{self, ErrorKind, Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::time::{Duration, SystemTime};

pub type Client = rustls::StreamOwned<rustls::ClientConnection, TcpStream>;

/// The third element is whether the server ACK'd `message-tags` (T135): a
/// `tail` caller uses this to read the real msgid off a pushed PRIVMSG
/// instead of inferring one, falling back to exactly its old polling
/// behaviour when this is `false` -- a NAK, or an older/unrelated IRC server
/// that never answers CAP at all.
/// Whether a read failed only because its `set_read_timeout` ran out, with the
/// connection still alive. The same event surfaces as `WouldBlock` on unix
/// and as `TimedOut` on Windows, and every polling loop here treats it as
/// "nothing yet, keep waiting"; checking one kind ended the `tail` loop, and so
/// the whole `tail`, at its first quiet second on Windows.
pub fn is_timeout(e: &io::Error) -> bool {
    matches!(e.kind(), ErrorKind::WouldBlock | ErrorKind::TimedOut)
}

pub fn connect(
    server: &str,
    nick: &str,
    state_dir: &std::path::Path,
    insecure: bool,
) -> Result<(Client, String, bool), String> {
    let addr = resolve(server)?;
    let tcp = TcpStream::connect(addr).map_err(|e| format!("connect {}: {}", server, e))?;
    tcp.set_nodelay(true).ok();
    // Bound the handshake and reads so a stalled server cannot hang the client.
    tcp.set_read_timeout(Some(Duration::from_secs(5))).ok();

    let conn = rustls::ClientConnection::new(
        Arc::new(client_config(insecure)),
        server_name(server, &addr)?,
    )
    .map_err(|e| format!("tls client: {}", e))?;
    let mut tls = rustls::StreamOwned::new(conn, tcp);

    // Register first: the first write drives the TLS handshake, after which
    // the peer certificate is available for TOFU pinning. One retry covers a
    // transient race with a just-started server.
    //
    // CAP LS goes out ahead of NICK/USER, in the same retry loop, as a real
    // IRCv3 client's first write: the server holds registration on CAP
    // LS/REQ until CAP END (T133), so the hold has to start before NICK/USER
    // reach it, not after.
    let mut attempts = 0;
    loop {
        let res = write_line(&mut tls, "CAP LS")
            .and_then(|_| write_line(&mut tls, "CAP REQ :message-tags"))
            .and_then(|_| write_line(&mut tls, &format!("NICK {}", nick)))
            .and_then(|_| write_line(&mut tls, &format!("USER {} 0 * :{}", nick, nick)));
        match res {
            Ok(()) => break,
            Err(e) => {
                if attempts >= 1 {
                    return Err(e.to_string());
                }
                attempts += 1;
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }

    // Read the CAP reply before ending negotiation: whatever a 433 (nickname
    // in use) reply from the NICK line above needs is unaffected either way
    // -- this only consumes CAP's own reply lines, so a 433 behind them in
    // the stream reaches `wait_for_welcome`'s own read untouched.
    let message_tags = negotiate_message_tags(&mut tls);
    let _ = write_line(&mut tls, "CAP END");

    // TOFU: verify the certificate fingerprint against the pinned one, or persist
    // on first connect (unless --insecure).
    let fp = cert_fingerprint(&tls)?;
    if !insecure {
        check_or_pin(server, &fp, state_dir)?;
    }
    Ok((tls, fp, message_tags))
}

/// Read CAP replies until message-tags is ACK'd, NAK'd, or a 2s deadline
/// passes with no CAP reply at all -- the "does not speak CAP" case is a
/// timeout, not a single read, because it is indistinguishable on the wire
/// from "slow".
fn negotiate_message_tags(tls: &mut Client) -> bool {
    let deadline = SystemTime::now() + Duration::from_secs(2);
    while SystemTime::now() < deadline {
        match read_line(tls) {
            Ok(l) => {
                if l.contains("CAP") && l.contains("ACK") && l.contains("message-tags") {
                    return true;
                }
                if l.contains("CAP") && l.contains("NAK") {
                    return false;
                }
            }
            Err(e) => {
                if !is_timeout(&e) {
                    return false;
                }
            }
        }
    }
    false
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

/// The host part of a `host:port` server address.
///
/// IPv6 needs care that splitting on the FIRST colon does not survive: a
/// bracketed `[::1]:6667` yielded `"["` and a bare `::1:6667` yielded the
/// empty string, and neither is a name anything can dial (B118). Both forms
/// are read the way std's `to_socket_addrs` reads them -- brackets stripped,
/// otherwise a trailing numeric port removed at the LAST colon -- so the host
/// this returns is the host the connect resolves to. IPv4 and hostnames carry
/// at most one colon, so they take the same path they always did.
pub fn server_host(server: &str) -> String {
    let s = server.trim();
    if let Ok(sa) = s.parse::<SocketAddr>() {
        return sa.ip().to_string();
    }
    if let Some(rest) = s.strip_prefix('[') {
        // Bracketed but not a whole socket address: no port, or an unusable one.
        return match rest.find(']') {
            Some(end) => rest[..end].to_string(),
            None => rest.to_string(),
        };
    }
    match s.rsplit_once(':') {
        Some((host, port)) if port.chars().all(|c| c.is_ascii_digit()) => host.to_string(),
        _ => s.to_string(),
    }
}

/// The TLS server name to present for a server address, given the socket
/// address the connect resolved to. An IP literal is not a DNS name, so
/// `ServerName::try_from` rejects it -- an IPv4 literal only passes by the
/// luck of looking like a label. An IP goes to `ServerName::IpAddress`
/// instead, taking the address from the resolved socket so the name always
/// matches what is dialled. A hostname keeps the DNS-name path it always had.
pub(crate) fn server_name(server: &str, addr: &SocketAddr) -> Result<ServerName<'static>, String> {
    let host = server_host(server);
    if host.parse::<std::net::IpAddr>().is_ok() {
        return Ok(ServerName::IpAddress(addr.ip().into()));
    }
    ServerName::try_from(host).map_err(|e| format!("server name: {}", e))
}

pub(crate) fn resolve(server: &str) -> Result<SocketAddr, String> {
    let mut it = server
        .to_socket_addrs()
        .map_err(|e| format!("resolve {}: {}", server, e))?;
    it.next()
        .ok_or_else(|| format!("no address for {}", server))
}

pub fn write_line(
    tls: &mut rustls::StreamOwned<rustls::ClientConnection, TcpStream>,
    line: &str,
) -> Result<(), String> {
    tls.write_all(format!("{}\r\n", line).as_bytes())
        .map_err(|e| format!("send: {}", e))?;
    tls.flush().map_err(|e| format!("flush: {}", e))?;
    Ok(())
}

pub fn read_line(
    tls: &mut rustls::StreamOwned<rustls::ClientConnection, TcpStream>,
) -> io::Result<String> {
    let mut buf = Vec::new();
    let mut b = [0u8; 1];
    loop {
        let n = tls.read(&mut b)?;
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
        return Err(io::Error::new(
            ErrorKind::UnexpectedEof,
            "connection closed by server",
        ));
    }
    Ok(String::from_utf8_lossy(&buf)
        .trim_end_matches(['\r', '\n'])
        .to_string())
}

pub fn wait_for_welcome(
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
                if !is_timeout(&e) {
                    return Err(e.to_string());
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
