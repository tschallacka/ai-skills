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
//! chat send  #chan "text" [-n NICK] [--host H] [--port N] [--home D]
//! chat read  #chan [--since N | --last N | --all] [--host H] [--port N] [--home D]
//! chat tail  #chan [--since N] [--host H] [--port N] [--home D]
//! chat register #chan [--host H] [--port N] [--home D]
//! ```
//!
//! And it inherits the helpers' central split: **without `--host` the client
//! never speaks to a server at all.** `send` appends under the channel lock and
//! `read`/`tail` read the log directly, exactly as `chat-send.sh` and
//! `chat-read.sh` do, which is why they keep working when no runtime can open a
//! socket. With `--host` the operation goes over the socket so a *remote* store
//! receives it.
//!
//! That split is also half of requirement 3. A client reaches a debug instance
//! only by being told its address: there is no probe, no scan, and no read of
//! anything a debug instance writes.

use crate::store::{valid_nick, Channel, Store};
use std::io::{BufRead, BufReader, Write};
use std::net::TcpStream;
use std::path::Path;
use std::time::Duration;

/// Where a client operation should act.
pub enum Target<'a> {
    /// The log under this home, with no server involved.
    Local(&'a Path),
    /// A server at this address. Named explicitly by the caller, always.
    Remote { host: &'a str, port: u16 },
}

/// The port the helpers assume when `--host` is given without `--port`.
/// Kept identical to them on purpose; see `instance::DEFAULT_PORT`.
pub const CLIENT_DEFAULT_PORT: u16 = crate::instance::DEFAULT_PORT;

/// A socket with timeouts set.
///
/// Timeouts are not decoration: without a read timeout a client whose server
/// dies mid-reply hangs forever, and an agent waiting on it hangs with it. Ten
/// seconds is far longer than a loopback exchange and far shorter than a stuck
/// agent being noticed.
fn connect(host: &str, port: u16) -> Result<TcpStream, String> {
    let s =
        TcpStream::connect((host, port)).map_err(|e| format!("cannot reach {host}:{port}: {e}"))?;
    let t = Some(Duration::from_secs(10));
    s.set_read_timeout(t).map_err(|e| e.to_string())?;
    s.set_write_timeout(t).map_err(|e| e.to_string())?;
    Ok(s)
}

/// Send `lines` and hand back a reader over the replies.
fn exchange(
    host: &str,
    port: u16,
    lines: &[String],
) -> Result<(TcpStream, BufReader<TcpStream>), String> {
    let mut sock = connect(host, port)?;
    let reader = sock
        .try_clone()
        .map(BufReader::new)
        .map_err(|e| e.to_string())?;
    for l in lines {
        writeln!(sock, "{l}").map_err(|e| format!("cannot write to {host}:{port}: {e}"))?;
    }
    sock.flush().map_err(|e| e.to_string())?;
    Ok((sock, reader))
}

/// `chat register #chan` — create the log so the channel exists before anyone
/// reads it. Locally this is a file create; remotely it is the REGISTER verb.
pub fn register(chan: &Channel, target: &Target) -> Result<String, String> {
    match target {
        Target::Local(home) => {
            let store = Store::new(home).map_err(|e| e.to_string())?;
            store.register(chan)?;
            Ok(format!("OK register {}", chan.as_str()))
        }
        Target::Remote { host, port } => {
            let (_s, reader) = exchange(
                host,
                *port,
                &[format!("REGISTER {}", chan.as_str()), "QUIT".to_string()],
            )?;
            for line in reader.lines() {
                let line = line.map_err(|e| e.to_string())?;
                if let Some(e) = line.strip_prefix("ERR ") {
                    return Err(e.to_string());
                }
                if line.starts_with("OK register") {
                    return Ok(line);
                }
            }
            Err(format!("no acknowledgement from {host}:{port}"))
        }
    }
}

/// `chat send #chan "text"` — returns the stored line, so the caller learns the
/// id that was actually allocated rather than assuming one.
pub fn send(chan: &Channel, nick: &str, text: &str, target: &Target) -> Result<String, String> {
    if !valid_nick(nick) {
        return Err(format!("invalid nick: {nick}"));
    }
    if text.is_empty() {
        return Err("empty message".to_string());
    }
    match target {
        Target::Local(home) => {
            let store = Store::new(home).map_err(|e| e.to_string())?;
            store.append(chan, nick, text)
        }
        Target::Remote { host, port } => {
            let (_s, reader) = exchange(
                host,
                *port,
                &[
                    format!("NICK {nick}"),
                    format!("PRIVMSG {} :{}", chan.as_str(), text),
                    "QUIT".to_string(),
                ],
            )?;
            for line in reader.lines() {
                let line = line.map_err(|e| e.to_string())?;
                if let Some(e) = line.strip_prefix("ERR ") {
                    return Err(e.to_string());
                }
                // The stored line for *our* channel is the acknowledgement. A
                // push for another channel could arrive on this socket first,
                // so match on the channel rather than on the first MSG seen.
                if is_msg_for(&line, chan) {
                    return Ok(line);
                }
            }
            Err(format!("no acknowledgement from {host}:{port}"))
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
pub fn read(chan: &Channel, range: &Range, target: &Target) -> Result<Vec<String>, String> {
    let rows = match target {
        Target::Local(home) => {
            let store = Store::new(home).map_err(|e| e.to_string())?;
            store.fetch(chan, 0)
        }
        Target::Remote { host, port } => fetch_remote(chan, 0, host, *port)?,
    };
    Ok(match range {
        // Filtering here rather than in the FETCH keeps one code path for both
        // targets, and --last has no wire form to ask for anyway.
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
pub fn tail(
    chan: &Channel,
    since: u64,
    target: &Target,
    out: &mut dyn Write,
) -> Result<(), String> {
    match target {
        Target::Local(home) => {
            let store = Store::new(home).map_err(|e| e.to_string())?;
            let mut seen = since;
            loop {
                for row in store.fetch(chan, seen) {
                    if let Some(id) = msg_id(&row) {
                        seen = seen.max(id);
                    }
                    writeln!(out, "{row}").map_err(|e| e.to_string())?;
                }
                out.flush().map_err(|e| e.to_string())?;
                std::thread::sleep(Duration::from_millis(500));
            }
        }
        Target::Remote { host, port } => {
            let mut sock = connect(host, *port)?;
            // No read timeout for a tail: idling is the normal state, and a
            // timeout would end the subscription every ten quiet seconds.
            sock.set_read_timeout(None).map_err(|e| e.to_string())?;
            let reader = sock
                .try_clone()
                .map(BufReader::new)
                .map_err(|e| e.to_string())?;
            // JOIN before FETCH, so a message arriving between the two is
            // pushed rather than missed. It may then appear twice, which the id
            // filter below removes — a duplicate is recoverable, a gap is not.
            writeln!(sock, "JOIN {}", chan.as_str()).map_err(|e| e.to_string())?;
            writeln!(sock, "FETCH {} {}", chan.as_str(), since).map_err(|e| e.to_string())?;
            sock.flush().map_err(|e| e.to_string())?;
            let mut seen = since;
            for line in reader.lines() {
                let line = line.map_err(|e| e.to_string())?;
                if let Some(e) = line.strip_prefix("ERR ") {
                    return Err(e.to_string());
                }
                if !is_msg_for(&line, chan) {
                    continue;
                }
                match msg_id(&line) {
                    Some(id) if id > seen => seen = id,
                    Some(_) => continue, // already printed
                    None => continue,
                }
                writeln!(out, "{line}").map_err(|e| e.to_string())?;
                out.flush().map_err(|e| e.to_string())?;
            }
            Err(format!("{host}:{port} closed the connection"))
        }
    }
}

fn fetch_remote(chan: &Channel, since: u64, host: &str, port: u16) -> Result<Vec<String>, String> {
    let (_s, reader) = exchange(
        host,
        port,
        &[
            format!("FETCH {} {}", chan.as_str(), since),
            "QUIT".to_string(),
        ],
    )?;
    let mut rows = Vec::new();
    for line in reader.lines() {
        let line = line.map_err(|e| e.to_string())?;
        if let Some(e) = line.strip_prefix("ERR ") {
            return Err(e.to_string());
        }
        if line.starts_with("OK fetch end") {
            return Ok(rows);
        }
        if is_msg_for(&line, chan) {
            rows.push(line);
        }
    }
    Err(format!("{host}:{port} closed before the fetch ended"))
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

    #[test]
    fn the_client_default_port_matches_the_default_server() {
        // If these ever drift, a default client silently talks to nothing.
        assert_eq!(CLIENT_DEFAULT_PORT, crate::instance::DEFAULT_PORT);
        assert_eq!(CLIENT_DEFAULT_PORT, 7717);
    }
}
