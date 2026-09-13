// MODE: DEV
// PACKAGE: PROD
//! The CLI verbs: one front end's argument handling, not the client's
//! interface (`chat-mcp` is the other front end and does not go through any
//! of this). Also the tail's control-socket request servicing for the other
//! verbs (T107's "tail owns the connection" design), which belongs beside the
//! verbs it serves rather than beside `control.rs` itself, which stays the
//! protocol/state module with zero knowledge of any specific verb.
//! (T101 split out of lib.rs).

use crate::control;
use crate::discovery::{resolve_server, DEFAULT_BEACON_PORT};
use crate::local::{channels_home, local_last_id, local_read};
use crate::net::{connect, read_line, wait_for_welcome, write_line, Client};
use crate::session::{
    apply_session, client_state_dir, parse_flag, save_cursor, save_session, session_key, Session,
};
use crate::wire::{json_field, mentions, msg_line_id, valid_chan, wire_segments};
use chat_proto::Message;
use std::collections::{HashMap, VecDeque};
use std::fs;
use std::io::ErrorKind;
use std::net::TcpStream;
use std::time::{Duration, Instant, SystemTime};

fn usage() {
    eprintln!(
        "chat-client-rs\n\n\
         connecting: a server may already be running -- every subcommand below\n\
         finds it on its own (session show state, then a UDP announce beacon)\n\
         with NO --server needed. To connect, just run:\n\
         \x20 chat-client-rs join --chan #c --nick N\n\
         --server/--insecure are a last resort for a server your beacon cannot\n\
         reach (a different host, a firewalled network) -- reaching for them\n\
         first is how two agents split one channel into two rival servers.\n\n\
         usage:\n\
         \x20 chat-client-rs discover [--wait S] [--beacon-port N] [--bcast ADDR] [--json]\n\
         \x20 chat-client-rs join  --chan #c [--nick N] [--since ID] [--server HOST:PORT] [--insecure]\n\
         \x20 chat-client-rs send  --chan #c --text MSG [--nick N] [--server HOST:PORT] [--insecure]\n\
         \x20 chat-client-rs read  --chan #c [--since ID] [--mentions] [--nick N] [--server HOST:PORT] [--insecure]\n\
         \x20 chat-client-rs read  --local --chan #c [--since ID] [--mentions] [--nick N]\n\
         \x20 chat-client-rs tail  --chan #c [--mentions] [--mention-exit] [--nick N] [--server HOST:PORT] [--insecure]\n\
         \x20 chat-client-rs tail  --local --chan #c [--mentions] [--mention-exit] [--nick N]\n\
         \x20 chat-client-rs leave --chan #c [--nick N] [--server HOST:PORT] [--insecure]\n\
         \x20 chat-client-rs names --chan #c [--nick N] [--server HOST:PORT] [--insecure]\n\
         \x20 chat-client-rs session show|set|clear|cursor\n\n\
         options (after the subcommand, unless noted):\n\
         \x20 --state DIR     client state dir, beats $AI_CHAT_HOME (default: $AI_CHAT_HOME\n\
         \x20                 or the tsch-ai-skills XDG chat dir)\n\
         \x20 --session ID    which session this agent owns (default: inferred, see below);\n\
         \x20                 the one flag also accepted BEFORE the subcommand\n\
         \x20 --server H:P    connect to this address instead of discovering one (last resort)\n\
         \x20 --insecure      do not pin the server cert (testing; last resort)\n\
         \x20 --no-session    ignore the saved session (server/nick/cursor)\n\
         \x20 --mentions      only messages mentioning your nick; tail filters pushed\n\
         \x20                 messages locally, read backfills server-side\n\
         \x20 --mention-exit  tail: exit as soon as a message mentions your nick\n\
         \x20 --local         maintenance escape hatch: read/tail the shared log,\n\
         \x20                 with no server. Ignores --state: the channel logs are the\n\
         \x20                 server's shared storage, --state is one client's own\n\n\
         The session remembers the default server+nick and per-channel cursors\n\
         (last seen message id). join seeds the cursor to the channel's current\n\
         end (so read/tail never dump old history); leave PARTs and drops the\n\
         cursor. A malformed session file is reset with a warning.\n\n\
         Each agent gets its own session file, so several agents can share one\n\
         AI_CHAT_HOME without sharing a nick or a cursor. Which session an\n\
         invocation owns is the first of these that applies:\n\
         \x20 1. --session ID, else $CHAT_SESSION_ID\n\
         \x20 2. a session id the harness exports (Claude Code, codex, opencode)\n\
         \x20 3. the git worktree root\n\
         \x20 4. otherwise one shared session\n\
         `session show` prints which rung decided and which file it is."
    );
    std::process::exit(64);
}

pub fn run() {
    let mut args: Vec<String> = std::env::args().collect();
    // --session is a global option, so it is accepted before the subcommand as
    // well as after it. session_key() reads it straight out of argv either way;
    // dropping the pair here keeps it from being read as the subcommand.
    if args.len() > 2 && args[1] == "--session" {
        args.drain(1..3);
    }
    if args.len() < 2 {
        usage();
    }
    let state_dir = client_state_dir(&args[2..]);
    match args[1].as_str() {
        "discover" => discover(&args[2..]),
        "send" => send(&args[2..], &state_dir),
        "read" => read_delta(&args[2..], &state_dir),
        "tail" => tail(&args[2..], &state_dir),
        "join" => join_channel(&args[2..], &state_dir),
        "names" => names(&args[2..], &state_dir),
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
            let (key, source) = session_key();
            println!("session={} source={}", key, source.as_str());
            println!("file={}", Session::path(state_dir).display());
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

pub(crate) struct Opts {
    pub(crate) server: String,
    pub(crate) nick: String,
    /// The first `--chan`. Every single-channel verb reads this, so they are
    /// unaffected by the flag becoming repeatable.
    pub(crate) chan: String,
    /// Every `--chan`, in the order given. Only `tail` reads this, because it
    /// is the only verb that follows channels rather than acting on one.
    pub(crate) chans: Vec<String>,
    pub(crate) text: String,
    pub(crate) since: String,
    pub(crate) insecure: bool,
    pub(crate) no_session: bool,
    pub(crate) mentions: bool,
    pub(crate) mention_exit: bool,
    // Show JOIN/PART/QUIT as well as messages. Off by default: every existing
    // reader of `tail` receives PRIVMSG only, and widening that silently would
    // change what they all see (B246).
    pub(crate) presence: bool,
    pub(crate) local: bool,
}

pub(crate) fn parse_opts(args: &[String]) -> Opts {
    let mut o = Opts {
        server: String::new(),
        nick: String::new(),
        chan: String::new(),
        chans: Vec::new(),
        text: String::new(),
        since: String::new(),
        insecure: false,
        no_session: false,
        mentions: false,
        mention_exit: false,
        presence: false,
        local: false,
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
            // Repeatable, for `tail`, which follows a set (T112). `chan` keeps
            // the FIRST one so every single-channel verb reads what it always
            // did; a second `--chan` to one of those is ignored rather than
            // silently changing which channel it acts on.
            "--chan" | "-c" => {
                i += 1;
                let name = args.get(i).cloned().unwrap_or_default();
                if o.chan.is_empty() {
                    o.chan = name.clone();
                }
                if !name.is_empty() && !o.chans.iter().any(|c| c == &name) {
                    o.chans.push(name);
                }
            }
            "--text" => {
                i += 1;
                o.text = args.get(i).cloned().unwrap_or_default();
            }
            "--since" => {
                i += 1;
                o.since = args.get(i).cloned().unwrap_or_default();
            }
            "--session" => {
                // Consumed by session_key() straight from argv; taken here too
                // so its value is not mistaken for another option.
                i += 1;
            }
            "--insecure" => o.insecure = true,
            "--no-session" => o.no_session = true,
            "--mentions" => o.mentions = true,
            "--mention-exit" => o.mention_exit = true,
            "--presence" => o.presence = true,
            "--local" => o.local = true,
            _ => {}
        }
        i += 1;
    }
    o
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

fn send(args: &[String], state_dir: &std::path::Path) {
    let o = parse_opts(args);
    // Send on the tail's connection when this session has one (T107). That is
    // what stops the server seeing a second registration under this nick and
    // suffixing it, which is how an agent's own messages arrived from
    // `<nick>-2` (B283).
    forward_or_fall_back(&o, state_dir, "send");
    let (mut server, nick, used_session) =
        apply_session(&o.server, &o.nick, state_dir, o.no_session);
    let from_session = server.clone();
    server = resolve_server(&o.server, &from_session, state_dir, o.no_session);
    if server.is_empty() {
        eprintln!(
            "chat-client-rs: no chat server found; pass --server HOST:PORT, or run `chat-client-rs session set --server HOST:PORT --nick NAME` (nothing answered: no --server, no saved session, no known server, no beacon)"
        );
        std::process::exit(64);
    }
    let session_current = used_session && server == from_session;
    if server.is_empty() || nick.is_empty() || o.chan.is_empty() || o.text.is_empty() {
        eprintln!("chat-client-rs: send needs --server --nick --chan --text (or a saved session)");
        std::process::exit(64);
    }
    let (mut tls, _fp, _message_tags) = match connect(&server, &nick, state_dir, o.insecure) {
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
    // No JOIN. A post does not imply presence, and the server's PRIVMSG handler
    // never checked the SENDER's membership -- it appends and relays, and
    // Peer::offer gates on the RECIPIENT being joined -- so this JOIN bought
    // nothing and cost a join/quit pair per call. Harmless while nothing could
    // see it; once membership was relayed (B244/B245) it made every send
    // flicker the nick in every other client's list (B247).
    // One PRIVMSG per wire segment, and the write result is CHECKED.
    //
    // B266: this was a single write of the whole --text with the result thrown
    // away by `let _ =`. write_line appends CRLF, so an embedded newline ended
    // the IRC line early and the server read the remainder as a command of its
    // own and discarded it. sanitairkamer measured ~1900-character messages
    // arriving as 415, 257 and less -- each cut exactly at its first paragraph
    // break -- and nothing anywhere reported a failure.
    let segments = wire_segments(&nick, &o.chan, &o.text);
    for segment in &segments {
        if let Err(e) = write_line(&mut tls, &format!("PRIVMSG {} :{}", o.chan, segment)) {
            eprintln!("chat-client-rs: {}", e);
            std::process::exit(70);
        }
    }
    // ASK for the acknowledgement rather than expecting one to be pushed. The
    // server sends the sender nothing after a PRIVMSG, deliberately: an echo
    // rendered twice in a standard client (B249), and an unsolicited numeric
    // rendered as a stray `[999] nick #chan 59` line, which is no better. 999
    // is LASTID's own reply, so asking makes the confirmation invisible to any
    // client that does not ask.
    let _ = write_line(&mut tls, &format!("LASTID {}", o.chan));
    // Wait for the 999 acknowledgement, not an echo of the message.
    //
    // The server used to write the PRIVMSG back to its sender, which is not the
    // RFC flow -- a client renders its own line locally -- so a standard client
    // showed every message twice (B249). It confirms with `999 <nick> #chan
    // <id>` now, which is a better signal anyway: it proves the line was
    // PERSISTED and names the id, where an echo only proved it was reflected.
    let mut acked = false;
    let deadline = SystemTime::now() + Duration::from_secs(4);
    while SystemTime::now() < deadline {
        match read_line(&mut tls) {
            Ok(l) => {
                if let Ok(m) = Message::parse(&l) {
                    if m.command == "999" && m.params.iter().any(|p| p == &o.chan) {
                        acked = true;
                        break;
                    }
                }
            }
            Err(e) => {
                if e.kind() != ErrorKind::WouldBlock {
                    break;
                }
            }
        }
    }
    if !acked {
        eprintln!("chat-client-rs: server did not acknowledge the message");
        std::process::exit(70);
    }
    // Printed locally, from what was SENT -- one line per wire segment, not one
    // line holding the whole --text. The server no longer echoes, so this print
    // is the only thing a caller sees, and printing the full text while the wire
    // carried a prefix of it is exactly how B266 stayed invisible: the sender's
    // own success output was the argument it passed, never the bytes that left.
    for segment in &segments {
        println!(":{nick}!{nick}@localhost PRIVMSG {} :{}", o.chan, segment);
    }
    // The cursor is NOT advanced here: sending is not reading (B254). A cursor
    // is one watermark over a shared channel, and no id skips only your own --
    // other agents' ids interleave with yours -- so advancing it past your own
    // message marks theirs as seen too.
    let _ = write_line(&mut tls, "QUIT");
}

/// Read the server's `:server 999 <nick> #chan <id>` reply (current max id).
pub fn read_last_id(
    tls: &mut rustls::StreamOwned<rustls::ClientConnection, TcpStream>,
    chan: &str,
    pending: &mut VecDeque<String>,
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
                pending.push_back(l);
            }
            Err(e) => {
                if e.kind() != ErrorKind::WouldBlock {
                    break;
                }
            }
        }
    }
    0
}

/// Join a channel without reading its history: seed the session cursor to the
/// channel's current end so later read/tail resume from "now".
// `names --chan #c` asks who is on a channel and prints them, one per line.
//
// The server has answered NAMES since it was written and nothing on the client
// ever asked (B256), so "is that peer listening right now?" was answered by
// guessing. A standard IRC client gets the list on join and keeps it live from
// the relayed JOIN/PART/QUIT; an agent has neither, and this gives it the same
// answer on demand.
//
// It deliberately does NOT join: NAMES reads the channel map and needs no
// membership, so asking who is present does not make the asker present. Same
// reasoning that took the JOIN out of `send` (B247) - a query is not a presence
// claim.
fn names(args: &[String], state_dir: &std::path::Path) {
    let o = parse_opts(args);
    forward_or_fall_back(&o, state_dir, "names");
    let (mut server, nick, used_session) =
        apply_session(&o.server, &o.nick, state_dir, o.no_session);
    let from_session = server.clone();
    server = resolve_server(&o.server, &from_session, state_dir, o.no_session);
    if server.is_empty() || nick.is_empty() || o.chan.is_empty() {
        eprintln!("chat-client-rs: names needs --server --nick --chan (or a saved session)");
        std::process::exit(64);
    }
    let session_current = used_session && server == from_session;
    let (mut tls, _fp, _message_tags) = match connect(&server, &nick, state_dir, o.insecure) {
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
    let _ = write_line(&mut tls, &format!("NAMES {}", o.chan));

    // 353 carries the members, 366 ends the list. Read until the end numeric, so
    // a server splitting 353 over several lines is handled.
    let mut members: Vec<String> = Vec::new();
    let deadline = SystemTime::now() + Duration::from_secs(4);
    let mut ended = false;
    while SystemTime::now() < deadline && !ended {
        match read_line(&mut tls) {
            Ok(l) => {
                if let Ok(m) = Message::parse(&l) {
                    match m.command.as_str() {
                        "353" => {
                            if let Some(list) = m.trailing.as_deref() {
                                members.extend(list.split_whitespace().map(|n| n.to_string()));
                            }
                        }
                        "366" => ended = true,
                        _ => {}
                    }
                }
            }
            Err(e) => {
                if e.kind() != ErrorKind::WouldBlock {
                    break;
                }
            }
        }
    }
    let _ = write_line(&mut tls, "QUIT");
    if !ended {
        eprintln!("chat-client-rs: no end-of-names from server");
        std::process::exit(70);
    }
    // An empty channel prints nothing and exits 0: "nobody is here" is an
    // answer, not a failure, and a caller distinguishes it by the empty output.
    for m in members {
        println!("{m}");
    }
}

fn join_channel(args: &[String], state_dir: &std::path::Path) {
    let o = parse_opts(args);
    forward_or_fall_back(&o, state_dir, "join");
    let (mut server, nick, used_session) =
        apply_session(&o.server, &o.nick, state_dir, o.no_session);
    let from_session = server.clone();
    server = resolve_server(&o.server, &from_session, state_dir, o.no_session);
    if server.is_empty() {
        eprintln!(
            "chat-client-rs: no chat server found; pass --server HOST:PORT, or run `chat-client-rs session set --server HOST:PORT --nick NAME` (nothing answered: no --server, no saved session, no known server, no beacon)"
        );
        std::process::exit(64);
    }
    let session_current = used_session && server == from_session;
    if server.is_empty() || nick.is_empty() || o.chan.is_empty() {
        eprintln!("chat-client-rs: join needs --server --nick --chan (or a saved session)");
        std::process::exit(64);
    }
    let (mut tls, _fp, _message_tags) = match connect(&server, &nick, state_dir, o.insecure) {
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
    let mut pending = VecDeque::new();
    let current = read_last_id(&mut tls, &o.chan, &mut pending);
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
    forward_or_fall_back(&o, state_dir, "leave");
    let (mut server, nick, used_session) =
        apply_session(&o.server, &o.nick, state_dir, o.no_session);
    let from_session = server.clone();
    server = resolve_server(&o.server, &from_session, state_dir, o.no_session);
    if server.is_empty() {
        eprintln!(
            "chat-client-rs: no chat server found; pass --server HOST:PORT, or run `chat-client-rs session set --server HOST:PORT --nick NAME` (nothing answered: no --server, no saved session, no known server, no beacon)"
        );
        std::process::exit(64);
    }
    let session_current = used_session && server == from_session;
    if server.is_empty() || nick.is_empty() || o.chan.is_empty() {
        eprintln!("chat-client-rs: leave needs --server --nick --chan (or a saved session)");
        std::process::exit(64);
    }
    let (mut tls, _fp, _message_tags) = match connect(&server, &nick, state_dir, o.insecure) {
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
    // If this session's tail holds the connection, read on it (T107). Returns
    // here when nothing is holding one, and the rest of the function is then
    // exactly what it always was.
    forward_or_fall_back(&o, state_dir, "read");
    if o.local {
        if o.chan.is_empty() {
            eprintln!("chat-client-rs: read --local needs --chan #c");
            std::process::exit(64);
        }
        if !valid_chan(&o.chan) {
            eprintln!(
                "chat-client-rs: read --local: not a channel name: {}",
                o.chan
            );
            std::process::exit(64);
        }
        // --since wins; otherwise the session cursor; otherwise everything.
        // A local read defaults to the whole log rather than the current end:
        // without a server there is no LASTID to ask, and silently printing
        // nothing is the worst of the available answers.
        let since = if !o.since.is_empty() {
            o.since.parse::<u64>().unwrap_or(0)
        } else if o.no_session {
            0
        } else {
            Session::load(state_dir).cursor(&o.chan)
        };
        let mentions_for = if o.mentions {
            let nick = if o.nick.is_empty() {
                Session::load(state_dir).nick
            } else {
                o.nick.clone()
            };
            if nick.is_empty() {
                eprintln!("chat-client-rs: --mentions needs --nick (or a saved session nick)");
                std::process::exit(64);
            }
            Some(nick)
        } else {
            None
        };
        let max_id = local_read(&channels_home(), &o.chan, since, mentions_for.as_deref());
        // A mention-filtered read must NOT move the channel cursor. The cursor
        // means "the last message I have seen in this channel", and a
        // `--mentions` read has seen only the mentions: `local_read` returns the
        // highest id it PRINTED, so saving it declares every non-mention message
        // below that id as seen. With the last mention at the end of the log
        // (`ping @bob` at id 4 over three unread messages) the cursor jumped
        // straight to 4 and the next plain read returned nothing at all -- the
        // three messages were skipped permanently, having been printed by
        // nothing. Leaving the cursor alone costs a mentions reader nothing: it
        // passes `--since` or reads the whole log by design.
        if mentions_for.is_none() && max_id > 0 {
            save_cursor(state_dir, &o.chan, max_id, o.no_session);
        }
        return;
    }
    let (mut server, nick, used_session) =
        apply_session(&o.server, &o.nick, state_dir, o.no_session);
    let from_session = server.clone();
    server = resolve_server(&o.server, &from_session, state_dir, o.no_session);
    if server.is_empty() {
        eprintln!(
            "chat-client-rs: no chat server found; pass --server HOST:PORT, or run `chat-client-rs session set --server HOST:PORT --nick NAME` (nothing answered: no --server, no saved session, no known server, no beacon)"
        );
        std::process::exit(64);
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
        // A RECORDED cursor is used even when it is 0 (B269). 0 means this
        // agent joined while the channel was empty, so id 1 onward is new to
        // it; treating that as "no cursor" skipped to the current end and lost
        // every message posted since the join.
        if let Some(cur) = Session::load(state_dir).cursor_recorded(&o.chan) {
            since = cur.to_string();
        }
    }
    let (mut tls, _fp, _message_tags) = match connect(&server, &nick, state_dir, o.insecure) {
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
        let mut pending = VecDeque::new();
        let current = read_last_id(&mut tls, &o.chan, &mut pending);
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
                if e.kind() != ErrorKind::WouldBlock {
                    break;
                }
            }
        }
    }
    // B295: a mention-filtered read must not move the cursor, the same reason
    // the --local arm above guards its own save. Saving the id of the last
    // MENTION printed marks every plain message below it as seen, though
    // nothing printed them.
    if !o.mentions && max_id > 0 {
        save_cursor(state_dir, &o.chan, max_id, o.no_session);
    }
    let _ = write_line(&mut tls, "QUIT");
}

// ---- serving a borrowed verb (T107) ---------------------------------------
// The owning tail performs these on ITS connection, from its own loop, so no
// second registration ever happens under the session's nick. Each one is the
// same wire exchange the standalone verb performs -- deliberately, because a
// caller cannot tell whether it was forwarded or fell back, and two paths that
// answer differently would make that difference visible as a bug.

/// What to do with a line read while collecting an answer.
enum Take {
    /// Part of the answer being collected.
    Keep,
    /// Not part of it: hand it back to the tail loop.
    Park,
    /// The answer is complete.
    Done,
}

/// Read until `classify` says the answer is complete, keeping what it selects
/// and parking everything else for the tail loop.
///
/// Parking is not tidiness. This is the tail's stream: a pushed PRIVMSG that
/// lands in the middle of a borrowed FETCH would otherwise be read here and
/// dropped -- the tail would never print it, and the cursor would still move
/// past it, so the message would be gone rather than late.
fn collect_answer(
    tls: &mut Client,
    pending: &mut VecDeque<String>,
    seconds: u64,
    mut classify: impl FnMut(&str) -> Take,
) -> (Vec<String>, bool) {
    let mut kept = Vec::new();
    let deadline = SystemTime::now() + Duration::from_secs(seconds);
    while SystemTime::now() < deadline {
        match read_line(tls) {
            Ok(line) => match classify(&line) {
                Take::Keep => kept.push(line),
                Take::Park => pending.push_back(line),
                Take::Done => return (kept, true),
            },
            Err(e) => {
                if e.kind() != ErrorKind::WouldBlock {
                    break;
                }
            }
        }
    }
    (kept, false)
}

/// Perform one borrowed verb and answer it.
/// The channels this tail follows, and whether it should stop.
///
/// Owned by the tail loop and handed to a borrowed verb by mutable reference:
/// `join` adds to it, `leave` removes from it, and emptying it stops the tail.
/// No lock, because serving happens on the tail's own thread -- the monitor
/// thread only queues the request.
struct Following {
    chans: Vec<String>,
    stop: bool,
}

/// Perform one borrowed verb and answer it.
fn serve_request(
    request: &control::Request,
    tls: &mut Client,
    nick: &str,
    following: &mut Following,
    state_dir: &std::path::Path,
    pending: &mut VecDeque<String>,
) -> control::Reply {
    if !request.chan.is_empty() && !valid_chan(&request.chan) {
        return control::Reply::fail(
            64,
            format!("chat-client-rs: not a channel name: {}", request.chan),
        );
    }
    match request.verb.as_str() {
        "send" => serve_send(request, tls, nick, pending),
        "read" => serve_read(request, tls, state_dir, pending),
        "names" => serve_names(request, tls, pending),
        "join" => serve_join(request, tls, following, state_dir, pending),
        "leave" => serve_leave(request, tls, following, state_dir),
        other => control::Reply::fail(
            64,
            format!("chat-client-rs: the session owner cannot serve {}", other),
        ),
    }
}

fn serve_send(
    request: &control::Request,
    tls: &mut Client,
    nick: &str,
    pending: &mut VecDeque<String>,
) -> control::Reply {
    let segments = wire_segments(nick, &request.chan, &request.text);
    for segment in &segments {
        if let Err(e) = write_line(tls, &format!("PRIVMSG {} :{}", request.chan, segment)) {
            return control::Reply::fail(70, format!("chat-client-rs: {}", e));
        }
    }
    if let Err(e) = write_line(tls, &format!("LASTID {}", request.chan)) {
        return control::Reply::fail(70, format!("chat-client-rs: {}", e));
    }
    // The 999 acknowledgement proves the line was PERSISTED. Nothing is
    // reported as sent without it: a send that only reached a socket buffer
    // reporting success is how B266 stayed invisible for as long as it did.
    if read_last_id(tls, &request.chan, pending) == 0 {
        return control::Reply::fail(70, "chat-client-rs: server did not acknowledge the message");
    }
    // Built from the segments that went to the wire, not from the text
    // argument, for the same reason the standalone path is (B266).
    //
    // The cursor is NOT advanced: sending is not reading (B254), and with one
    // owner there is now exactly one process that could have advanced it.
    control::Reply::ok(
        segments
            .iter()
            .map(|segment| {
                format!(
                    ":{nick}!{nick}@localhost PRIVMSG {} :{}",
                    request.chan, segment
                )
            })
            .collect(),
    )
}

fn serve_read(
    request: &control::Request,
    tls: &mut Client,
    state_dir: &std::path::Path,
    pending: &mut VecDeque<String>,
) -> control::Reply {
    let mut since = request.since.clone();
    if since.is_empty() {
        // A RECORDED cursor counts even at 0 (B269).
        if let Some(cursor) = Session::load(state_dir).cursor_recorded(&request.chan) {
            since = cursor.to_string();
        }
    }
    if since.is_empty() {
        if let Err(e) = write_line(tls, &format!("LASTID {}", request.chan)) {
            return control::Reply::fail(70, format!("chat-client-rs: {}", e));
        }
        since = read_last_id(tls, &request.chan, pending).to_string();
    }
    let suffix = if request.mentions { " mentions" } else { "" };
    if let Err(e) = write_line(tls, &format!("FETCH {} {}{}", request.chan, since, suffix)) {
        return control::Reply::fail(70, format!("chat-client-rs: {}", e));
    }
    let (lines, _) = collect_answer(tls, pending, 5, |line| {
        if line.starts_with(":server 000 end-of-history") {
            Take::Done
        } else if line.starts_with("MSG ") {
            Take::Keep
        } else {
            Take::Park
        }
    });
    let max_id = lines.iter().filter_map(|line| msg_line_id(line)).max();
    // B295: not saved for a mention-filtered read, matching the standalone
    // remote read (now fixed the same way) and the --local arm's own guard.
    if !request.mentions {
        if let Some(id) = max_id {
            save_cursor(state_dir, &request.chan, id, false);
        }
    }
    control::Reply::ok(lines)
}

fn serve_names(
    request: &control::Request,
    tls: &mut Client,
    pending: &mut VecDeque<String>,
) -> control::Reply {
    if let Err(e) = write_line(tls, &format!("NAMES {}", request.chan)) {
        return control::Reply::fail(70, format!("chat-client-rs: {}", e));
    }
    let (lines, ended) = collect_answer(tls, pending, 4, |line| {
        match Message::parse(line).map(|m| m.command) {
            Ok(command) if command == "353" => Take::Keep,
            Ok(command) if command == "366" => Take::Done,
            _ => Take::Park,
        }
    });
    if !ended {
        return control::Reply::fail(70, "chat-client-rs: no end-of-names from server");
    }
    let members = lines
        .iter()
        .filter_map(|line| Message::parse(line).ok())
        .filter_map(|message| message.trailing)
        .flat_map(|list| {
            list.split_whitespace()
                .map(str::to_string)
                .collect::<Vec<String>>()
        })
        .collect();
    control::Reply::ok(members)
}

fn serve_join(
    request: &control::Request,
    tls: &mut Client,
    following: &mut Following,
    state_dir: &std::path::Path,
    pending: &mut VecDeque<String>,
) -> control::Reply {
    if let Err(e) = write_line(tls, &format!("JOIN {}", request.chan))
        .and_then(|_| write_line(tls, &format!("LASTID {}", request.chan)))
    {
        return control::Reply::fail(70, format!("chat-client-rs: {}", e));
    }
    let current = read_last_id(tls, &request.chan, pending);
    let seed = if request.since.is_empty() {
        current
    } else {
        request.since.parse::<u64>().unwrap_or(current)
    };
    let mut session = Session::load(state_dir);
    session.cursors.insert(request.chan.clone(), seed);
    let _ = session.save(state_dir);
    // The JOIN above made this connection a MEMBER of the channel, so the
    // server will now relay its traffic here. Following it is not optional: a
    // tail that stayed on its original channel would read those lines off the
    // stream and drop them, leaving the agent in a member list it cannot hear
    // and a cursor that never moves (B296). Membership and listening are the
    // same decision, so they are made in the same place.
    if !following.chans.iter().any(|c| c == &request.chan) {
        following.chans.push(request.chan.clone());
    }
    control::Reply::ok(vec![format!(
        "joined {} (resuming after id {}); following {}",
        request.chan,
        seed,
        following.chans.join(", ")
    )])
}

fn serve_leave(
    request: &control::Request,
    tls: &mut Client,
    following: &mut Following,
    state_dir: &std::path::Path,
) -> control::Reply {
    if let Err(e) = write_line(tls, &format!("PART {}", request.chan)) {
        return control::Reply::fail(70, format!("chat-client-rs: {}", e));
    }
    let mut session = Session::load(state_dir);
    session.cursors.remove(&request.chan);
    let _ = session.save(state_dir);
    // Leaving a channel this tail follows stops FOLLOWING it. An earlier
    // version refused the request when the channel was the tailed one and told
    // the caller to stop the tail instead -- which is the wrong end of the
    // choice, because the caller asked to leave and there is nothing ambiguous
    // about that (T112).
    let was_following = following.chans.iter().any(|c| c == &request.chan);
    following.chans.retain(|c| c != &request.chan);
    let mut out = vec![format!("left {}", request.chan)];
    if was_following && following.chans.is_empty() {
        // Nothing left to follow. A tail that kept running here would hold a
        // connection subscribed to no channel while still answering as the
        // session's owner: present nowhere, waking on nothing, and looking
        // alive to anything reading the socket.
        following.stop = true;
        out.push("no channels left to follow; the tail is stopping".to_string());
    } else if was_following {
        out.push(format!("still following {}", following.chans.join(", ")));
    }
    control::Reply::ok(out)
}

/// Ask the session's owner to perform this verb, and finish the process with
/// what it reports.
///
/// It RETURNS when there is no owner to ask, and the caller then opens its own
/// connection exactly as it did before T107. Every reason for having no owner
/// is one of those: no tail running, a tail that could not bind, a socket left
/// by a dead one, or an owner whose identity does not match an explicit
/// `--nick`/`--server`.
///
/// `--no-session` means "touch no shared state", so it never forwards: the
/// owner's connection and cursor file are precisely that shared state.
fn forward_or_fall_back(o: &Opts, state_dir: &std::path::Path, verb: &str) {
    if o.no_session || o.local || o.chan.is_empty() {
        return;
    }
    if verb == "send" && o.text.is_empty() {
        return;
    }
    let request = control::Request {
        verb: verb.to_string(),
        chan: o.chan.clone(),
        text: o.text.clone(),
        since: o.since.clone(),
        mentions: o.mentions,
        nick: o.nick.clone(),
        server: o.server.clone(),
    };
    let reply = match control::ask(state_dir, &session_key().0, &request) {
        Some(reply) => reply,
        None => return,
    };
    for line in &reply.out {
        println!("{}", line);
    }
    if !reply.err.is_empty() {
        eprintln!("{}", reply.err);
    }
    std::process::exit(reply.code);
}

pub(crate) fn tail(args: &[String], state_dir: &std::path::Path) {
    let o = parse_opts(args);
    // `--mention-exit` means "stop once a message mentions me", which needs the
    // mention filter to know what a mention is. Alone it was quietly wrong in
    // opposite directions on the two paths: the socket tail guards the exit with
    // `o.mentions`, so the flag did nothing at all, while the local tail's exit
    // sat outside that guard and returned on the FIRST message from anyone.
    // Refusing the combination is the only reading that cannot surprise either
    // way; a caller that meant "wake me on a mention" was always going to pass
    // both.
    if o.mention_exit && !o.mentions {
        eprintln!("chat-client-rs: tail --mention-exit needs --mentions");
        std::process::exit(64);
    }
    if o.local {
        // A local tail walks ONE log file, so it follows one channel. Refused
        // rather than quietly following the first of several: a caller that
        // passed three names and got one channel's traffic has no way to tell
        // that from a quiet bus.
        if o.chans.len() > 1 {
            eprintln!(
                "chat-client-rs: tail --local follows one channel; got {}",
                o.chans.join(", ")
            );
            std::process::exit(64);
        }
        if o.chan.is_empty() {
            eprintln!("chat-client-rs: tail --local needs --chan #c");
            std::process::exit(64);
        }
        if !valid_chan(&o.chan) {
            eprintln!(
                "chat-client-rs: tail --local: not a channel name: {}",
                o.chan
            );
            std::process::exit(64);
        }
        let nick = if o.nick.is_empty() {
            Session::load(state_dir).nick
        } else {
            o.nick.clone()
        };
        if o.mentions && nick.is_empty() {
            eprintln!("chat-client-rs: --mentions needs --nick (or a saved session nick)");
            std::process::exit(64);
        }
        let mentions_for = if o.mentions {
            Some(nick.as_str())
        } else {
            None
        };
        // A local tail starts at the log's current end unless told otherwise:
        // a watcher wants what arrives next, not the backlog.
        let mut since = if !o.since.is_empty() {
            o.since.parse::<u64>().unwrap_or(0)
        } else {
            local_last_id(&channels_home(), &o.chan)
        };
        // Same step-down cadence as the socket tail: responsive while a
        // conversation is live, near-silent when nothing is happening.
        // Record where the watch begins, not just what it later sees. Skipping
        // the backlog is a decision this tail makes for the whole channel, so
        // until it is written down `read --local` still believes nothing has
        // been seen: a tail that started at the log end and then idled left no
        // session file at all, and the next plain read re-printed the entire
        // backlog the tail had just declared old. A mention-filtered tail is the
        // exception, for the reason `read --local` does not save one either.
        if mentions_for.is_none() && since > 0 {
            save_cursor(state_dir, &o.chan, since, o.no_session);
        }
        let mut wait = 1u64;
        loop {
            let max_id = local_read(&channels_home(), &o.chan, since, mentions_for);
            if max_id > since {
                since = max_id;
                if mentions_for.is_none() {
                    save_cursor(state_dir, &o.chan, max_id, o.no_session);
                }
                wait = 1;
                // Guarded on the filter, matching the socket tail. Unguarded,
                // this returned on the first message from anyone; `tail()` now
                // also refuses `--mention-exit` without `--mentions`, so the
                // two agree and this cannot fire on a non-mention.
                if o.mention_exit && mentions_for.is_some() {
                    return;
                }
            } else if wait < 60 {
                wait = (wait * 2).min(60);
            }
            std::thread::sleep(Duration::from_secs(wait));
        }
    }
    let (mut server, nick, used_session) =
        apply_session(&o.server, &o.nick, state_dir, o.no_session);
    let from_session = server.clone();
    server = resolve_server(&o.server, &from_session, state_dir, o.no_session);
    if server.is_empty() {
        eprintln!(
            "chat-client-rs: no chat server found; pass --server HOST:PORT, or run `chat-client-rs session set --server HOST:PORT --nick NAME` (nothing answered: no --server, no saved session, no known server, no beacon)"
        );
        std::process::exit(64);
    }
    let session_current = used_session && server == from_session;
    if server.is_empty() || nick.is_empty() || o.chans.is_empty() {
        eprintln!("chat-client-rs: tail needs --server --nick --chan (or a saved session)");
        std::process::exit(64);
    }
    // One tail follows a SET of channels: `--chan` is repeatable. Every name is
    // checked before anything is joined, so a typo in the third does not leave
    // the first two half-followed.
    for chan in &o.chans {
        if !valid_chan(chan) {
            eprintln!("chat-client-rs: tail: not a channel name: {}", chan);
            std::process::exit(64);
        }
    }
    let (mut tls, _fp, message_tags) = match connect(&server, &nick, state_dir, o.insecure) {
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
    // Resume from the session cursor; with NO cursor recorded, default to the
    // channel's CURRENT end (LASTID) so tailing an old channel does not dump
    // its whole history — only new messages are shown from now on.
    //
    // A recorded cursor (map, per channel) is replayed via FETCH below
    // (B269): 0 is a position, not an absence, and JOIN's push starts from
    // now, so history since the last run would otherwise be lost for good.
    let mut pending = VecDeque::new();
    let mut cursors: HashMap<String, u64> = HashMap::new();
    for chan in &o.chans {
        let _ = write_line(&mut tls, &format!("JOIN {}", chan));
        let seed = if o.no_session {
            0
        } else {
            match Session::load(state_dir).cursor_recorded(chan) {
                Some(cur) => {
                    let suffix = if o.mentions { " mentions" } else { "" };
                    let _ = write_line(&mut tls, &format!("FETCH {} {}{}", chan, cur, suffix));
                    let deadline = SystemTime::now() + Duration::from_secs(5);
                    let mut max_id = cur;
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
                                } else {
                                    // A live push interleaved with the fetch
                                    // reply on this same socket; parked for
                                    // the tail loop below, not dropped.
                                    pending.push_back(l);
                                }
                            }
                            Err(e) => {
                                if e.kind() != ErrorKind::WouldBlock {
                                    break;
                                }
                            }
                        }
                    }
                    max_id
                }
                None => {
                    let _ = write_line(&mut tls, &format!("LASTID {}", chan));
                    read_last_id(&mut tls, chan, &mut pending)
                }
            }
        };
        cursors.insert(chan.clone(), seed);
    }
    // JOIN subscribes this connection to the live PRIVMSG stream from here
    // on; the FETCH above is what covers everything before it, once per
    // channel at startup rather than on every steady-state push.
    //
    // The set is owned by the loop rather than read from `o`, because a
    // borrowed `join` or `leave` changes it while the tail runs.
    let mut following = Following {
        chans: o.chans.clone(),
        stop: false,
    };
    // One session, one connection (T107). From here this process OWNS the
    // session: send, read, names, join and leave forward their request over a
    // socket in the state dir and print what this connection reports, instead
    // of registering a second time under the same nick -- which is what the
    // server suffixed (B283), and what left two processes writing one cursor
    // (B254, B269).
    //
    // `--no-session` owns nothing. It means "touch no shared state", and the
    // socket plus the cursor file is exactly that shared state.
    //
    // A tail that cannot bind -- a second tail in the same session, a `--state`
    // path too long for a socket address, a state dir it cannot write -- simply
    // does not own the socket. It still tails, and every other verb still works
    // the way it did before any of this existed.
    let owner = if o.no_session {
        None
    } else {
        control::serve(
            state_dir,
            &session_key().0,
            control::OwnerRecord {
                server: server.clone(),
                nick: nick.clone(),
                chan: following.chans.join(","),
                ..control::OwnerRecord::default()
            },
        )
    };
    if let Some(owner) = &owner {
        // Owning the socket shortens this loop's own read deadline. The loop
        // only comes back around to take queued work when a read returns, so
        // at the 5s connect default a forwarded send waited up to five seconds
        // for a wire exchange that takes milliseconds. A shorter deadline costs
        // an idle tail one wakeup a second and nothing else: the server writes
        // each line in a single write, so a line is not split across a timeout.
        let _ = tls.sock.set_read_timeout(Some(Duration::from_secs(1)));
        eprintln!(
            "chat-client-rs: this session's verbs are served on {}",
            owner.socket().display()
        );
    }
    let mut last_sync = Instant::now();
    let mut sync_next = 0usize;
    loop {
        // Borrowed work first, and before blocking on a read: one atomic load
        // when there is nothing waiting, which is the overwhelming majority of
        // iterations. Each request is answered from this connection, so its
        // caller sees what the wire said rather than what it asked for.
        if let Some(owner) = &owner {
            for job in owner.take_pending() {
                let reply = serve_request(
                    &job.request,
                    &mut tls,
                    &nick,
                    &mut following,
                    state_dir,
                    &mut pending,
                );
                job.answer(reply);
            }
        }
        // A borrowed `leave` that took the last channel leaves this tail with
        // nothing to follow, and it stops. Staying up would hold a connection
        // subscribed to nothing while still answering as the session's owner --
        // present in no channel, waking on nothing, and looking alive.
        if following.stop {
            break;
        }
        match pending
            .pop_front()
            .map(Ok)
            .unwrap_or_else(|| read_line(&mut tls))
        {
            Ok(l) => {
                let message = match Message::parse(&l) {
                    Ok(message) => message,
                    Err(_) => continue,
                };
                // Membership lines, when asked for. The server relays JOIN,
                // PART and QUIT correctly (B244/B245), and this loop used to
                // drop every one of them before the mention filter ran -- so a
                // human with a standard client could see who was present and a
                // tailing agent could not (B246). "Is that peer listening right
                // now?" was unanswerable from the bus, which matters because
                // agents coordinate handoffs through it.
                //
                // A QUIT carries no channel parameter: it goes to every channel
                // the leaver shared, and the server only relays it to members of
                // this one, so an unfiltered command is already scoped.
                if o.presence && matches!(message.command.as_str(), "JOIN" | "PART" | "QUIT") {
                    let in_a_followed_chan = message.command == "QUIT"
                        || message
                            .params
                            .iter()
                            .any(|p| following.chans.iter().any(|c| c == p))
                        || message
                            .trailing
                            .as_deref()
                            .map(|t| following.chans.iter().any(|c| c == t))
                            .unwrap_or(false);
                    if in_a_followed_chan {
                        println!("{}", l);
                    }
                    continue;
                }
                let addressed_to = message.params.first().map(String::as_str).unwrap_or("");
                if message.command != "PRIVMSG"
                    || !following.chans.iter().any(|c| c == addressed_to)
                {
                    continue;
                }
                // T135: a message-tags-negotiated connection gets the real id
                // inline on this exact line, which is what B157 asked for --
                // no separate poll, no window where the cursor lags what was
                // just shown. Both the plain and the mention-filtered path
                // advance from it identically; only what gets PRINTED differs
                // between them, below.
                if message_tags {
                    if let Some(id) = message
                        .tags
                        .iter()
                        .find(|t| t.key == "msgid")
                        .and_then(|t| t.value.as_deref())
                        .and_then(|v| v.parse::<u64>().ok())
                    {
                        let recorded = cursors.entry(addressed_to.to_string()).or_insert(0);
                        if id > *recorded {
                            *recorded = id;
                            save_cursor(state_dir, addressed_to, id, o.no_session);
                        }
                    }
                }
                let is_mention = message
                    .trailing
                    .as_deref()
                    .map(|text| mentions(text, &nick))
                    .unwrap_or(false);
                // message-tags is this connection's own bookkeeping for the
                // cursor above; what gets PRINTED stays exactly the shape it
                // was before tags existed, so anything already parsing a
                // tail's stdout (this repo's own test suite included) does
                // not have to learn a new line shape it never asked for.
                let display = if message.tags.is_empty() {
                    l.clone()
                } else {
                    Message {
                        tags: Vec::new(),
                        prefix: message.prefix.clone(),
                        command: message.command.clone(),
                        params: message.params.clone(),
                        trailing: message.trailing.clone(),
                    }
                    .serialize()
                };
                if !o.mentions || is_mention {
                    if o.mentions {
                        println!("!! MENTION !! {}", display);
                    } else {
                        println!("{}", display);
                    }
                    if o.mention_exit && is_mention {
                        // Stop serving before leaving: the socket outlives the
                        // process otherwise, and a client that connects to it
                        // in the gap waits out its whole deadline for an owner
                        // that is gone. Removing it sends that client back to
                        // its own connection immediately.
                        if let Some(owner) = &owner {
                            control::stop(owner);
                        }
                        let _ = write_line(&mut tls, "QUIT");
                        std::process::exit(0);
                    }
                }
                // T135's fallback: without a negotiated msgid tag (NAK'd, or
                // an older/unrelated server that never answered CAP at all),
                // a pushed line carries no id, so resynchronize at a bounded
                // cadence from the server's authoritative maximum instead --
                // exactly this tail's behaviour before message-tags existed.
                // A negotiated connection never reaches here: every pushed
                // line already carried its own real id, above.
                if !message_tags
                    && last_sync.elapsed() >= Duration::from_secs(1)
                    && !following.chans.is_empty()
                {
                    // ONE channel per tick, round-robin. The cost of staying
                    // synchronized must not grow with the number of channels
                    // followed -- a tail on five channels would otherwise spend
                    // five round trips a second on bookkeeping -- and a cursor
                    // is a watermark that only moves forward, so reaching each
                    // channel every few seconds instead of every second loses
                    // nothing.
                    sync_next %= following.chans.len();
                    let chan = following.chans[sync_next].clone();
                    sync_next += 1;
                    let _ = write_line(&mut tls, &format!("LASTID {}", chan));
                    let authoritative_id = read_last_id(&mut tls, &chan, &mut pending);
                    let recorded = cursors.entry(chan.clone()).or_insert(0);
                    if authoritative_id > *recorded {
                        *recorded = authoritative_id;
                    }
                    save_cursor(state_dir, &chan, *recorded, o.no_session);
                    last_sync = Instant::now();
                }
            }
            Err(e) => {
                if e.kind() != ErrorKind::WouldBlock {
                    break;
                }
            }
        }
    }
    // QUIT so the server sees a departure rather than a dropped socket: a tail
    // that stopped because it was told to leave should look like it left. On a
    // broken stream this write fails and is ignored, which is the same outcome
    // as not attempting it.
    let _ = write_line(&mut tls, "QUIT");
    // The stream is gone, so nothing can be served on it any more. Taking the
    // socket down now is what lets the next verb fall back to its own
    // connection immediately, rather than wait out a deadline against an owner
    // that no longer has one.
    if let Some(owner) = &owner {
        control::stop(owner);
    }
}
