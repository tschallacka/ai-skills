// MODE: DEV
// PACKAGE: PROD
//! The line protocol, and the subscriber set that push delivery needs.
//!
//! The protocol is fixed by the five bash helpers and the interpreter tiers, so
//! this is a conforming reimplementation, not a redesign. One verb per line,
//! one reply per verb:
//!
//! ```text
//! NICK <nick>            -> OK nick <nick>          | ERR invalid nick
//! REGISTER #chan         -> OK register #chan       | ERR invalid channel
//! JOIN #chan             -> OK join #chan           | ERR invalid channel
//! LEAVE #chan            -> OK leave #chan
//! PRIVMSG #chan :text    -> the stored MSG line, and a push to other joiners
//! FETCH #chan <since-id> -> zero or more MSG lines, then OK fetch end
//! PING                   -> PONG
//! QUIT                   -> OK bye, then the connection closes
//! ```
//!
//! **A locally appended message is not pushed.** `chat-send.sh` without
//! `--host` writes the log directly under the channel lock, and nothing tells
//! the server, so a socket JOINer never sees it — it appears only on the next
//! `FETCH`. That is inherited behaviour, documented in `chat/SKILL.md`, and
//! deliberately not papered over here: watching the log for outside appends is
//! a design change, not a bug fix.

use crate::store::{valid_nick, Channel, Joined, Store};
use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// A connected client, as seen by everyone else: somewhere to write, and the
/// channels it wants pushes for.
struct Peer {
    out: TcpStream,
    chans: Joined,
}

/// Shared state: the log, plus who is listening to what.
pub struct Hub {
    store: Store,
    peers: Mutex<HashMap<u64, Peer>>,
    next_peer: AtomicU64,
}

impl Hub {
    pub fn new(store: Store) -> Hub {
        Hub {
            store,
            peers: Mutex::new(HashMap::new()),
            next_peer: AtomicU64::new(1),
        }
    }

    /// Serve one connection to completion. Errors are reported to the client
    /// where the protocol has a reply for them and swallowed where it does not,
    /// because one client's broken pipe must not take the server down.
    pub fn serve(self: &Arc<Self>, stream: TcpStream) {
        let id = self.next_peer.fetch_add(1, Ordering::Relaxed);
        let reader = match stream.try_clone() {
            Ok(s) => BufReader::new(s),
            Err(_) => return,
        };
        let writer = match stream.try_clone() {
            Ok(s) => s,
            Err(_) => return,
        };
        {
            let mut peers = self.peers.lock().unwrap_or_else(|e| e.into_inner());
            peers.insert(
                id,
                Peer {
                    out: writer,
                    chans: Joined::new(),
                },
            );
        }
        // Documented default: anon-<n>, and it changes on every reconnect. It
        // is not unique — nick uniqueness is not enforced, by design.
        let mut nick = format!("anon-{id}");
        let mut out = stream;
        for raw in reader.lines() {
            let line = match raw {
                Ok(l) => l,
                Err(_) => break, // reset connection; nothing to reply to
            };
            let line = line.trim_end_matches(['\r', '\n']);
            if line.is_empty() {
                continue;
            }
            match self.dispatch(id, &mut nick, line, &mut out) {
                Ok(true) => {}
                Ok(false) => break, // QUIT
                Err(()) => break,   // the socket is gone
            }
        }
        let mut peers = self.peers.lock().unwrap_or_else(|e| e.into_inner());
        peers.remove(&id);
    }

    /// One verb. `Ok(false)` means the client asked to leave.
    fn dispatch(
        &self,
        id: u64,
        nick: &mut String,
        line: &str,
        out: &mut TcpStream,
    ) -> Result<bool, ()> {
        let (verb, arg) = match line.split_once(' ') {
            Some((v, rest)) => (v, rest.trim()),
            None => (line, ""),
        };
        match verb {
            "NICK" => {
                if valid_nick(arg) {
                    *nick = arg.to_string();
                    reply(out, &format!("OK nick {nick}"))
                } else {
                    reply(out, "ERR invalid nick")
                }
            }
            "REGISTER" => match Channel::parse(arg) {
                None => reply(out, "ERR invalid channel"),
                Some(c) => match self.store.register(&c) {
                    Ok(()) => reply(out, &format!("OK register {}", c.as_str())),
                    Err(e) => reply(out, &format!("ERR {e}")),
                },
            },
            "JOIN" => match Channel::parse(arg) {
                None => reply(out, "ERR invalid channel"),
                Some(c) => {
                    let mut peers = self.peers.lock().unwrap_or_else(|e| e.into_inner());
                    if let Some(p) = peers.get_mut(&id) {
                        p.chans.insert(c.clone());
                    }
                    drop(peers);
                    reply(out, &format!("OK join {}", c.as_str()))
                }
            },
            // Leaving a channel never joined is success: it is the state the
            // caller asked for, and the helpers call it unconditionally.
            "LEAVE" => {
                if let Some(c) = Channel::parse(arg) {
                    let mut peers = self.peers.lock().unwrap_or_else(|e| e.into_inner());
                    if let Some(p) = peers.get_mut(&id) {
                        p.chans.remove(&c);
                    }
                }
                reply(out, &format!("OK leave {arg}"))
            }
            "PRIVMSG" => self.privmsg(id, nick, arg, out),
            "FETCH" => self.fetch(arg, out),
            "PING" => reply(out, "PONG"),
            "QUIT" => {
                reply(out, "OK bye")?;
                Ok(false)
            }
            other => reply(out, &format!("ERR unknown verb {other}")),
        }
    }

    /// `PRIVMSG #chan :text`.
    ///
    /// B59: the two ways this can be malformed get two different messages. The
    /// python tier answers a bad channel with the empty-text usage line, which
    /// sends the caller looking at the wrong argument.
    fn privmsg(&self, id: u64, nick: &str, arg: &str, out: &mut TcpStream) -> Result<bool, ()> {
        let (chan_raw, text) = match arg.split_once(" :") {
            Some((c, t)) => (c, t),
            None => return reply(out, "ERR usage: PRIVMSG #chan :text"),
        };
        let chan = match Channel::parse(chan_raw) {
            Some(c) => c,
            None => return reply(out, "ERR invalid channel"),
        };
        if text.is_empty() {
            return reply(out, "ERR empty message: PRIVMSG #chan :text");
        }
        match self.store.append(&chan, nick, text) {
            Err(e) => reply(out, &format!("ERR {e}")),
            Ok(stored) => {
                // The sender is told what landed before anyone else is told at
                // all, so the acknowledgement cannot be lost behind a slow peer.
                reply(out, &stored)?;
                self.broadcast(&stored, &chan, id);
                Ok(true)
            }
        }
    }

    fn fetch(&self, arg: &str, out: &mut TcpStream) -> Result<bool, ()> {
        let (chan_raw, since_raw) = match arg.split_once(' ') {
            Some((c, s)) => (c, s.trim()),
            None => (arg, "0"),
        };
        let chan = match Channel::parse(chan_raw) {
            Some(c) => c,
            None => return reply(out, "ERR invalid channel"),
        };
        let since: u64 = match since_raw.parse() {
            Ok(n) => n,
            Err(_) => return reply(out, "ERR usage: FETCH #chan <since-id>"),
        };
        for row in self.store.fetch(&chan, since) {
            reply(out, &row)?;
        }
        reply(out, "OK fetch end")
    }

    /// Push to every joiner except the originator, which already has its
    /// acknowledgement. A peer whose socket has gone is skipped rather than
    /// removed here: its own thread notices and cleans up.
    fn broadcast(&self, line: &str, chan: &Channel, origin: u64) {
        let mut peers = self.peers.lock().unwrap_or_else(|e| e.into_inner());
        for (pid, peer) in peers.iter_mut() {
            if *pid == origin || !peer.chans.contains(chan) {
                continue;
            }
            let _ = writeln!(peer.out, "{line}");
            let _ = peer.out.flush();
        }
    }
}

/// One reply line. A write failure means the client is gone, which is not an
/// error worth logging once per line.
fn reply(out: &mut TcpStream, text: &str) -> Result<bool, ()> {
    writeln!(out, "{text}").map_err(|_| ())?;
    out.flush().map_err(|_| ())?;
    Ok(true)
}
