// MODE: DEV
// PACKAGE: PROD
//! The local channel-log reader: the maintenance escape hatch that reads a
//! channel straight off the server's shared log storage, with no server
//! connection (T101 split out of lib.rs).

use crate::session::chat_default_home;
use crate::wire::{mentions, msg_line_id};
use std::fs;
use std::io::BufRead;
use std::path::PathBuf;

pub fn local_chan_log(home: &std::path::Path, chan: &str) -> PathBuf {
    home.join("channels").join(format!("{}.log", chan))
}

/// Where the CHANNEL LOGS live, which is not the same place as `--state`.
///
/// Channel logs are the server's shared storage; `--state` is one client's own
/// certificate pins and sessions. An agent given its own `--state` directory
/// must still read the channels everyone shares, so a local read resolves the
/// home from `$AI_CHAT_HOME` (or the XDG default) exactly as the server does,
/// and ignores `--state`.
pub fn channels_home() -> PathBuf {
    std::env::var("AI_CHAT_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| chat_default_home())
}

/// The highest stored id in a channel log, or 0 when there is none.
///
/// Taken from the maximum over all rows rather than the last line: a truncated
/// or interleaved final write must not make the cursor go backwards.
pub fn local_last_id(home: &std::path::Path, chan: &str) -> u64 {
    let path = local_chan_log(home, chan);
    let file = match fs::File::open(&path) {
        Ok(f) => f,
        Err(_) => return 0,
    };
    let mut top = 0u64;
    for line in std::io::BufReader::new(file).lines().map_while(Result::ok) {
        if let Some(id) = msg_line_id(&line) {
            if id > top {
                top = id;
            }
        }
    }
    top
}

/// Print stored messages with id > `since`, straight from the channel log.
/// Returns the highest id printed.
///
/// This is the reader the skill was missing. Every read path required a live
/// server, so when none was reachable the only way to see a channel was to
/// open its log by hand -- which is what agents actually did, and it bypasses
/// mention filtering and cursors entirely. Reading the log is lossless: the
/// log IS the storage format, so a local read returns the same lines a FETCH
/// would.
pub(crate) fn local_read(
    home: &std::path::Path,
    chan: &str,
    since: u64,
    mentions_for: Option<&str>,
) -> u64 {
    let path = local_chan_log(home, chan);
    let file = match fs::File::open(&path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!(
                "chat-client-rs: cannot read {}: {} (no such channel locally; a channel exists once its first message is stored)",
                path.display(),
                e
            );
            std::process::exit(66);
        }
    };
    let mut max_id = 0u64;
    for line in std::io::BufReader::new(file).lines().map_while(Result::ok) {
        let id = match msg_line_id(&line) {
            Some(id) => id,
            None => continue, // a malformed line is skipped, never fatal
        };
        if id <= since {
            continue;
        }
        if let Some(nick) = mentions_for {
            if !mentions(&line, nick) {
                continue;
            }
        }
        println!("{}", line);
        if id > max_id {
            max_id = id;
        }
    }
    max_id
}
