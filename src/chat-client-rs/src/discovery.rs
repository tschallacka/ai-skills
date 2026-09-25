// MODE: DEV
// PACKAGE: PROD
//! Server discovery: the UDP announce beacon, the discovered-server cache,
//! and the resolution ladder that picks which server to dial (T101 split out
//! of lib.rs).

use crate::wire::json_field;
use std::fs;
use std::time::{Duration, SystemTime};

pub const DEFAULT_BEACON_PORT: u16 = 7780;

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
pub fn discover_candidates(beacon_port: u16, wait_s: u64) -> Vec<String> {
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
pub fn resolve_server(
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
