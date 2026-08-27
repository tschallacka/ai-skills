// MODE: DEV
// PACKAGE: PROD
//! The client verbs: register, send, read, tail.
//!
//! These exist because the five helpers are bash, and a bash-less host — a
//! Windows agent outside Git Bash, a scratch container with `/bin/sh` and
//! nothing else — could be handed a server it had no way to talk to. So the
//! same binary carries both sides.
//!
//! **The CLI deliberately mirrors the helpers**, flag for flag, so a command
//! written for one works against the other and `chat/SKILL.md` documents one
//! grammar rather than two:
//!
//! ```text
//! chat send  #chan "text" [-n NICK] [--host H] [--port N] [--socket P] [--local] [--home D]
//! chat read  #chan [--since N | --last N | --all] [...]
//! chat tail  #chan [--since N] [...]
//! chat register #chan [...]
//! ```
//!
//! Two ways to act, and which one is chosen is the whole of the transport
//! question seen from the client side:
//!
//! * **Local.** No server involved: `send` appends under the channel lock and
//!   `read`/`tail` read the log, exactly as `chat-send.sh` and `chat-read.sh` do
//!   without `--host`. This is why the clients keep working when nothing can
//!   open a socket at all.
//! * **Remote.** Over the transport, so a *remote* store receives it.
//!
//! Precedence for which: an explicit `--host`/`--port`/`--socket`/`--local`
//! beats the recorded config, and the config beats the built-in default. A
//! remote target that cannot be reached falls back to local with a note on
//! stderr — the fallback is announced rather than silent, because a down server
//! and a working one are not the same situation even when the outcome looks
//! alike.
//!
//! Requirement 3 lives here too: a debug instance is reachable only by being
//! told its address, because there is no probe, no scan, and no read of anything
//! a debug instance writes.

use crate::config::Transport;
use crate::net;
use crate::store::{valid_nick, Channel, Store};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::Path;
use std::time::Duration;

/// Where a client operation should act.
pub enum Target<'a> {
    /// The log under this home, with no server involved.
    Local(&'a Path),
    /// A server on this transport.
    Remote(Transport),
}

/// Failures a caller must tell apart.
///
/// `Unreachable` is the only one a local fallback is allowed to answer: a
/// protocol error means the server heard us and objected, and retrying that
/// locally would write something the server refused.
#[derive(Debug)]
pub enum ClientError {
    Unreachable(String),
    Failed(String),
}

impl ClientError {
    pub fn message(&self) -> &str {
        match self {
            ClientError::Unreachable(m) | ClientError::Failed(m) => m,
        }
    }
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        f.write_str(self.message())
    }
}

type Res<T> = Result<T, ClientError>;

fn failed<T, E: std::fmt::Display>(e: E) -> Res<T> {
    Err(ClientError::Failed(e.to_string()))
}

/// Send `lines` and hand back a reader over the replies.
fn exchange(transport: &Transport, lines: &[String]) -> Res<BufReader<Box<dyn Read + Send>>> {
    let mut client = net::connect(transport, false).map_err(ClientError::Unreachable)?;
    for l in lines {
        writeln!(client.write, "{l}").map_err(|e| ClientError::Failed(e.to_string()))?;
    }
    client
        .write
        .flush()
        .map_err(|e| ClientError::Failed(e.to_string()))?;
    Ok(BufReader::new(client.read))
}

/// `chat register #chan` — create the log so the channel exists before anyone
/// reads it. Locally this is a file create; remotely it is the REGISTER verb.
pub fn register(chan: &Channel, target: &Target) -> Res<String> {
    match target {
        Target::Local(home) => {
            let store = Store::new(home).or_else(failed)?;
            store.register(chan).or_else(failed)?;
            Ok(format!("OK register {}", chan.as_str()))
        }
        Target::Remote(t) => {
            let reader = exchange(t, &[format!("REGISTER {}", chan.as_str()), "QUIT".into()])?;
            for line in reader.lines() {
                let line = line.or_else(failed)?;
                if let Some(e) = line.strip_prefix("ERR ") {
                    return failed(e);
                }
                if line.starts_with("OK register") {
                    return Ok(line);
                }
            }
            Err(ClientError::Failed(format!("no acknowledgement from {t}")))
        }
    }
}

/// `chat send #chan "text"` — returns the stored line, so the caller learns the
/// id that was actually allocated rather than assuming one.
pub fn send(chan: &Channel, nick: &str, text: &str, target: &Target) -> Res<String> {
    if !valid_nick(nick) {
        return failed(format!("invalid nick: {nick}"));
    }
    if text.is_empty() {
        return failed("empty message");
    }
    match target {
        Target::Local(home) => {
            let store = Store::new(home).or_else(failed)?;
            store.append(chan, nick, text).or_else(failed)
        }
        Target::Remote(t) => {
            let reader = exchange(
                t,
                &[
                    format!("NICK {nick}"),
                    format!("PRIVMSG {} :{}", chan.as_str(), text),
                    "QUIT".into(),
                ],
            )?;
            for line in reader.lines() {
                let line = line.or_else(failed)?;
                if let Some(e) = line.strip_prefix("ERR ") {
                    return failed(e);
                }
                // The stored line for *our* channel is the acknowledgement. A
                // push for another channel could arrive on this socket first,
                // so match on the channel rather than on the first MSG seen.
                if is_msg_for(&line, chan) {
                    return Ok(line);
                }
            }
            Err(ClientError::Failed(format!("no acknowledgement from {t}")))
        }
    }
}

/// How many stored lines to hand back.
pub enum Range {
    Since(u64),
    Last(usize),
    All,
}

/// `chat read #chan` — the delta, the tail, or everything.
pub fn read(chan: &Channel, range: &Range, target: &Target) -> Res<Vec<String>> {
    // --since is the one range the wire understands, so ask for it rather than
    // pulling the whole log across and discarding most of it. --last has no wire
    // form, so it is still filtered here, over one FETCH from 0.
    let ask_from = match range {
        Range::Since(n) => *n,
        _ => 0,
    };
    let rows = match target {
        Target::Local(home) => {
            let store = Store::new(home).or_else(failed)?;
            store.fetch(chan, ask_from)
        }
        Target::Remote(t) => fetch_remote(chan, ask_from, t)?,
    };
    Ok(match range {
        // Already filtered by ask_from; the filter stays as the belt to that
        // braces, since a server is free to send more than was asked for.
        Range::Since(n) => rows
            .into_iter()
            .filter(|r| msg_id(r).is_some_and(|id| id > *n))
            .collect(),
        Range::Last(n) => {
            let skip = rows.len().saturating_sub(*n);
            rows.into_iter().skip(skip).collect()
        }
        Range::All => rows,
    })
}

/// `chat tail #chan` — print what is stored, then keep printing as it arrives.
///
/// Remotely this is a real subscription: JOIN, and the server pushes. Locally
/// there is nothing to push, so it polls the log the way `chat-tail.sh` does.
/// The two are not equivalent and the difference is visible: a message appended
/// locally by another process is not pushed to a socket subscriber (see
/// `wire.rs`), so a remote tail can lag a local writer until the next read.
pub fn tail(chan: &Channel, since: u64, target: &Target, out: &mut dyn Write) -> Res<()> {
    match target {
        Target::Local(home) => {
            let store = Store::new(home).or_else(failed)?;
            let mut seen = since;
            loop {
                for row in store.fetch(chan, seen) {
                    if let Some(id) = msg_id(&row) {
                        seen = seen.max(id);
                    }
                    writeln!(out, "{row}").or_else(failed)?;
                }
                out.flush().or_else(failed)?;
                std::thread::sleep(Duration::from_millis(500));
            }
        }
        Target::Remote(t) => {
            let mut client = net::connect(t, true).map_err(ClientError::Unreachable)?;
            let reader = BufReader::new(client.read);
            // JOIN before FETCH, so a message arriving between the two is
            // pushed rather than missed. It may then appear twice, which the id
            // filter below removes — a duplicate is recoverable, a gap is not.
            writeln!(client.write, "JOIN {}", chan.as_str()).or_else(failed)?;
            writeln!(client.write, "FETCH {} {}", chan.as_str(), since).or_else(failed)?;
            client.write.flush().or_else(failed)?;
            let mut seen = since;
            for line in reader.lines() {
                let line = line.or_else(failed)?;
                if let Some(e) = line.strip_prefix("ERR ") {
                    return failed(e);
                }
                if !is_msg_for(&line, chan) {
                    continue;
                }
                match msg_id(&line) {
                    Some(id) if id > seen => seen = id,
                    Some(_) => continue, // already printed
                    None => continue,
                }
                writeln!(out, "{line}").or_else(failed)?;
                out.flush().or_else(failed)?;
            }
            failed(format!("{t} closed the connection"))
        }
    }
}

fn fetch_remote(chan: &Channel, since: u64, transport: &Transport) -> Res<Vec<String>> {
    let reader = exchange(
        transport,
        &[format!("FETCH {} {}", chan.as_str(), since), "QUIT".into()],
    )?;
    let mut rows = Vec::new();
    for line in reader.lines() {
        let line = line.or_else(failed)?;
        if let Some(e) = line.strip_prefix("ERR ") {
            return failed(e);
        }
        if line.starts_with("OK fetch end") {
            return Ok(rows);
        }
        if is_msg_for(&line, chan) {
            rows.push(line);
        }
    }
    failed(format!("{transport} closed before the fetch ended"))
}

/// A stored line for this channel — checked on the channel field, not by
/// substring, so text mentioning another channel cannot be mistaken for one.
fn is_msg_for(line: &str, chan: &Channel) -> bool {
    let mut f = line.split(' ');
    f.next() == Some("MSG") && f.next() == Some(chan.as_str())
}

fn msg_id(line: &str) -> Option<u64> {
    let mut f = line.split(' ');
    if f.next()? != "MSG" {
        return None;
    }
    f.next()?;
    f.next()?.parse().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chan(n: &str) -> Channel {
        Channel::parse(n).unwrap()
    }

    fn home(tag: &str) -> std::path::PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("chat-client-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        p
    }

    #[test]
    fn a_local_send_is_readable_locally_with_no_server() {
        let h = home("local");
        let c = chan("#t");
        let stored = send(&c, "codex", "hello", &Target::Local(&h)).unwrap();
        assert!(stored.starts_with("MSG #t 1 "), "got: {stored}");
        let rows = read(&c, &Range::All, &Target::Local(&h)).unwrap();
        assert_eq!(rows, vec![stored]);
    }

    #[test]
    fn since_and_last_select_the_documented_slices() {
        let h = home("slices");
        let c = chan("#t");
        for t in ["a", "b", "c"] {
            send(&c, "n", t, &Target::Local(&h)).unwrap();
        }
        assert_eq!(
            read(&c, &Range::Since(1), &Target::Local(&h))
                .unwrap()
                .len(),
            2
        );
        assert_eq!(
            read(&c, &Range::Last(2), &Target::Local(&h)).unwrap().len(),
            2
        );
        assert_eq!(read(&c, &Range::All, &Target::Local(&h)).unwrap().len(), 3);
        // --last beyond the log is the whole log, not an error.
        assert_eq!(
            read(&c, &Range::Last(99), &Target::Local(&h))
                .unwrap()
                .len(),
            3
        );
    }

    #[test]
    fn an_invalid_nick_is_refused_before_anything_is_written() {
        let h = home("nick");
        let c = chan("#t");
        assert!(send(&c, "bad nick", "x", &Target::Local(&h)).is_err());
        assert!(
            read(&c, &Range::All, &Target::Local(&h))
                .unwrap()
                .is_empty(),
            "a refused send must not have written anything"
        );
    }

    #[test]
    fn an_empty_message_is_refused() {
        let h = home("empty");
        assert!(send(&chan("#t"), "n", "", &Target::Local(&h)).is_err());
    }

    /// The channel field is compared, not searched for. A message whose *text*
    /// names another channel must not be picked up as one.
    #[test]
    fn a_channel_is_matched_on_its_field_not_by_substring() {
        let c = chan("#t");
        assert!(is_msg_for("MSG #t 1 0 n :hi", &c));
        assert!(!is_msg_for("MSG #other 1 0 n :about #t", &c));
        assert!(!is_msg_for("OK join #t", &c));
    }

    /// A server nobody is running is Unreachable, which is the only error a
    /// local fallback may answer. A protocol refusal must never fall back, or a
    /// client would write locally the very thing the server rejected.
    #[test]
    fn a_dead_endpoint_reports_unreachable_and_not_a_protocol_failure() {
        // Port 1 on loopback: privileged, and nothing listens there.
        let t = Transport::Tcp {
            bind: "127.0.0.1".to_string(),
            port: 1,
        };
        match send(&chan("#t"), "n", "x", &Target::Remote(t)) {
            Err(ClientError::Unreachable(_)) => {}
            other => panic!("expected Unreachable, got {other:?}"),
        }
    }
}
