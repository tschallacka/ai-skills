// MODE: DEV
// PACKAGE: PROD
//! The message log: validation, id allocation, append and delta fetch.
//!
//! The log format and the lock are **not** this binary's private business —
//! `chat-send.sh` appends to the same file under the same lock, and
//! `chat-read.sh` / `chat-tail.sh` read it directly with `awk` and `tail`. So
//! the on-disk contract is fixed by those helpers, and this module conforms to
//! them rather than the other way round:
//!
//! * one file per channel at `<home>/channels/<chan>.log`
//! * one line per message, `MSG #chan <id> <ts> <nick> :text`
//! * mutual exclusion by `mkdir` on `<home>/channels/<chan>.lock`, because
//!   `mkdir` is the only atomic test-and-set a POSIX shell has. An `flock`
//!   here would be invisible to the helpers and would not exclude them.
//!
//! Three things are done deliberately differently from the interpreter tiers,
//! each because the difference is a filed defect there (B52, B56, B57). They
//! are noted at the point where they matter.

use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Matches the cap every server tier enforces. The client helpers do not
/// enforce it (B58), which is why a name is checked here rather than trusted.
const MAX_CHAN: usize = 33; // '#' plus 32
const MAX_NICK: usize = 32;
const LOCK_TRIES: u32 = 200;
const LOCK_WAIT: Duration = Duration::from_millis(50);

pub struct Store {
    chan_dir: PathBuf,
}

/// A channel name that has been validated, so no code below can be handed an
/// unchecked one — the reason the field is private.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Channel(String);

impl Channel {
    /// `#` followed by 1..=32 of lowercase, digit, `_` or `-`.
    ///
    /// Deliberately identical to `valid_chan` in every interpreter tier: a name
    /// one tier accepts and another rejects would make a channel readable
    /// locally and unreachable over the socket.
    pub fn parse(raw: &str) -> Option<Channel> {
        if raw.len() < 2 || raw.len() > MAX_CHAN || !raw.starts_with('#') {
            return None;
        }
        let ok = raw[1..]
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-');
        if ok {
            Some(Channel(raw.to_string()))
        } else {
            None
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// `1..=32` of alphanumeric, `_` or `-`, matching the tiers and `chat-send.sh`.
pub fn valid_nick(nick: &str) -> bool {
    !nick.is_empty()
        && nick.len() <= MAX_NICK
        && nick
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

impl Store {
    pub fn new(home: &Path) -> std::io::Result<Store> {
        let chan_dir = home.join("channels");
        fs::create_dir_all(&chan_dir)?;
        Ok(Store { chan_dir })
    }

    fn log_path(&self, chan: &Channel) -> PathBuf {
        self.chan_dir.join(format!("{}.log", chan.as_str()))
    }

    fn lock_path(&self, chan: &Channel) -> PathBuf {
        self.chan_dir.join(format!("{}.lock", chan.as_str()))
    }

    /// Create the log if it does not exist. Registering an existing channel is
    /// success, not an error: agents call it on every startup.
    pub fn register(&self, chan: &Channel) -> Result<(), String> {
        let _held = self.lock(chan)?;
        OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.log_path(chan))
            .map(|_| ())
            .map_err(|e| format!("cannot create the log for {}: {e}", chan.as_str()))
    }

    /// Allocate the next id, append, and return the stored line verbatim so the
    /// sender is told exactly what landed.
    pub fn append(&self, chan: &Channel, nick: &str, text: &str) -> Result<String, String> {
        let _held = self.lock(chan)?;
        let id = self.highest_id(chan) + 1;
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        // Newlines would forge a second message; the helpers fold them to
        // spaces with `tr`, so do the same and not something subtler.
        let flat: String = text
            .chars()
            .map(|c| if c == '\n' || c == '\r' { ' ' } else { c })
            .collect();
        let line = format!("MSG {} {} {} {} :{}", chan.as_str(), id, ts, nick, flat);
        let mut fh = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.log_path(chan))
            .map_err(|e| format!("cannot open the log for {}: {e}", chan.as_str()))?;
        writeln!(fh, "{line}").map_err(|e| format!("cannot append to {}: {e}", chan.as_str()))?;
        Ok(line)
    }

    /// The highest id in the log, **not** the id on the last line.
    ///
    /// B56: the python3 and node tiers read the last line, so once any line is
    /// out of order — and `chat-send.sh` appending concurrently is how that
    /// happens — the next message takes an id at or below one already stored,
    /// and every `--since` reader silently skips it.
    fn highest_id(&self, chan: &Channel) -> u64 {
        let fh = match File::open(self.log_path(chan)) {
            Ok(f) => f,
            Err(_) => return 0,
        };
        let mut high = 0u64;
        for line in BufReader::new(fh).lines() {
            // A read error is not a bad line, and skipping it does not make
            // progress: `lines()` over a path that is a directory yields
            // EISDIR forever, so `continue` here is an infinite loop. Found by
            // the B52 lock test, which hangs rather than fails against it.
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };
            // B57: an unparseable line must not be fatal. server.py lets the
            // int() raise, which kills the connection with no error reply, so
            // one corrupt byte in a log takes the channel down for everyone.
            if let Some(id) = parse_id(&line) {
                if id > high {
                    high = id;
                }
            }
        }
        high
    }

    /// Every stored line with an id greater than `since`, in file order.
    pub fn fetch(&self, chan: &Channel, since: u64) -> Vec<String> {
        let fh = match File::open(self.log_path(chan)) {
            Ok(f) => f,
            Err(_) => return Vec::new(),
        };
        let mut out = Vec::new();
        for line in BufReader::new(fh).lines() {
            // Stop, not skip: see highest_id. A read error repeats.
            let line = match line {
                Ok(l) => l,
                Err(_) => break,
            };
            if let Some(id) = parse_id(&line) {
                if id > since {
                    out.push(line);
                }
            }
        }
        out
    }

    /// Take the channel lock, or say why not.
    ///
    /// B52: the shell handler leaves the lock directory behind when the locked
    /// command fails, which wedges the channel for every writer until someone
    /// removes it by hand. Here the guard's `Drop` releases it on every path
    /// out — early return, `?`, or panic — which is the whole reason the lock
    /// is a value rather than a pair of calls.
    fn lock(&self, chan: &Channel) -> Result<LockGuard, String> {
        let path = self.lock_path(chan);
        for _ in 0..LOCK_TRIES {
            match fs::create_dir(&path) {
                Ok(()) => return Ok(LockGuard { path }),
                Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                    thread::sleep(LOCK_WAIT)
                }
                Err(e) => return Err(format!("cannot lock {}: {e}", chan.as_str())),
            }
        }
        Err(format!(
            "lock timeout on {} after {}s; if no writer is running, remove {}",
            chan.as_str(),
            LOCK_TRIES as u64 * LOCK_WAIT.as_millis() as u64 / 1000,
            path.display()
        ))
    }
}

struct LockGuard {
    path: PathBuf,
}

impl Drop for LockGuard {
    fn drop(&mut self) {
        let _ = fs::remove_dir(&self.path);
    }
}

/// The id from a stored line, or `None` if the line is not one we wrote.
fn parse_id(line: &str) -> Option<u64> {
    let mut fields = line.split(' ');
    if fields.next()? != "MSG" {
        return None;
    }
    let _chan = fields.next()?;
    fields.next()?.parse().ok()
}

/// What a channel set is called on the wire side; kept here so `Channel`'s
/// invariant travels with it.
pub type Joined = HashSet<Channel>;

#[cfg(test)]
mod tests {
    use super::*;

    fn store(tag: &str) -> (Store, PathBuf) {
        let mut home = std::env::temp_dir();
        home.push(format!("chat-store-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&home);
        (Store::new(&home).unwrap(), home)
    }

    fn chan(name: &str) -> Channel {
        Channel::parse(name).unwrap()
    }

    #[test]
    fn a_channel_name_matches_what_every_tier_accepts() {
        assert!(Channel::parse("#codegraph").is_some());
        assert!(Channel::parse("#a-b_9").is_some());
        assert!(Channel::parse("#").is_none(), "bare # has no name");
        assert!(Channel::parse("codegraph").is_none(), "missing #");
        assert!(Channel::parse("#Upper").is_none(), "uppercase");
        assert!(Channel::parse("#a b").is_none(), "space");
    }

    /// B58's cap, enforced server-side because the helpers do not enforce it.
    #[test]
    fn a_channel_name_over_the_cap_is_refused() {
        let ok = format!("#{}", "a".repeat(32));
        let too_long = format!("#{}", "a".repeat(33));
        assert!(Channel::parse(&ok).is_some(), "32 is the documented cap");
        assert!(Channel::parse(&too_long).is_none());
    }

    #[test]
    fn nicks_follow_the_same_charset_as_the_send_helper() {
        assert!(valid_nick("codex"));
        assert!(valid_nick("agent-1_b"));
        assert!(!valid_nick(""));
        assert!(!valid_nick("has space"));
        assert!(!valid_nick(&"n".repeat(33)));
    }

    #[test]
    fn append_stores_the_documented_line_format() {
        let (s, _h) = store("format");
        let line = s.append(&chan("#t"), "codex", "hello").unwrap();
        let f: Vec<&str> = line.splitn(6, ' ').collect();
        assert_eq!(f[0], "MSG");
        assert_eq!(f[1], "#t");
        assert_eq!(f[2], "1");
        assert!(f[3].parse::<u64>().is_ok(), "timestamp: {}", f[3]);
        assert_eq!(f[4], "codex");
        assert_eq!(f[5], ":hello");
    }

    #[test]
    fn ids_increment_across_appends() {
        let (s, _h) = store("ids");
        let c = chan("#t");
        assert!(s.append(&c, "a", "one").unwrap().contains(" 1 "));
        assert!(s.append(&c, "a", "two").unwrap().contains(" 2 "));
        assert!(s.append(&c, "a", "three").unwrap().contains(" 3 "));
    }

    /// The B56 regression. The highest id is 9, but it is not on the last
    /// line — which is exactly what a concurrent `chat-send.sh` append
    /// produces. An implementation reading the last line allocates 6 and
    /// overwrites a live id.
    #[test]
    fn the_next_id_comes_from_the_highest_not_the_last_line() {
        let (s, home) = store("highest");
        let c = chan("#t");
        s.register(&c).unwrap();
        let log = home.join("channels/#t.log");
        fs::write(
            &log,
            "MSG #t 9 100 a :ninth\nMSG #t 5 101 b :fifth arrived late\n",
        )
        .unwrap();
        let line = s.append(&c, "a", "next").unwrap();
        assert!(
            line.contains(" 10 "),
            "should follow the highest id, got: {line}"
        );
    }

    /// The B57 regression. server.py raises on this log and drops the client.
    #[test]
    fn a_corrupt_line_does_not_stop_the_channel() {
        let (s, home) = store("corrupt");
        let c = chan("#t");
        s.register(&c).unwrap();
        let log = home.join("channels/#t.log");
        fs::write(&log, "MSG #t notanumber 100 a :junk\nMSG #t 3 101 b :real\n").unwrap();
        let line = s
            .append(&c, "a", "after")
            .expect("a corrupt line must not make the channel unwritable");
        assert!(line.contains(" 4 "), "got: {line}");
        let rows = s.fetch(&c, 0);
        assert_eq!(rows.len(), 2, "the unparseable line is skipped, not fatal");
    }

    #[test]
    fn fetch_returns_only_ids_above_since() {
        let (s, _h) = store("since");
        let c = chan("#t");
        for n in ["a", "b", "c"] {
            s.append(&c, "n", n).unwrap();
        }
        assert_eq!(s.fetch(&c, 0).len(), 3);
        assert_eq!(s.fetch(&c, 2).len(), 1);
        assert_eq!(s.fetch(&c, 9).len(), 0);
    }

    #[test]
    fn fetch_on_an_unknown_channel_is_empty_not_an_error() {
        let (s, _h) = store("unknown");
        assert!(s.fetch(&chan("#nope"), 0).is_empty());
    }

    /// The B52 regression: the lock must be gone after a failed operation, or
    /// the channel is wedged for every writer including the shell helpers.
    #[test]
    fn the_channel_lock_is_released_even_when_the_operation_fails() {
        let (s, home) = store("lockfail");
        let c = chan("#t");
        // Make the append fail for a reason that is not the lock: the log path
        // is a directory, so opening it for append cannot succeed.
        fs::create_dir_all(home.join("channels/#t.log")).unwrap();
        assert!(s.append(&c, "a", "x").is_err(), "append should have failed");
        assert!(
            !home.join("channels/#t.lock").exists(),
            "a failed append left the lock behind, wedging the channel"
        );
    }

    #[test]
    fn a_newline_in_the_text_cannot_forge_a_second_message() {
        let (s, _h) = store("newline");
        let c = chan("#t");
        let line = s.append(&c, "a", "one\nMSG #t 99 0 evil :two").unwrap();
        assert!(!line.contains('\n'), "stored line must stay one line");
        assert_eq!(s.fetch(&c, 0).len(), 1);
    }
}
