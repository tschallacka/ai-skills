// MODE: DEV
// PACKAGE: PROD
//! The held connection: one TLS session, owned by one thread, for the life of
//! the adapter process.
//!
//! A one-shot CLI can hold nothing: it connects, does one thing and exits, so
//! there is no presence and no push. One thread here owns the stream for the
//! process's life, registers once and keeps draining, so an arriving message is
//! already in hand when `wait` asks for it.
//!
//! The thread owns the stream rather than sharing it under a mutex because
//! reading and writing a `rustls` stream are the same object: a blocking read
//! held across a tool call would lock out the write that ends it. Interleaving
//! both in one loop — take an operation, or read for a tick — is also what
//! keeps the socket drained while no tool call is in flight. The server drops a
//! subscriber whose outbox passes a megabyte, and that is indistinguishable
//! from message loss.

use chat_client_rs as client;
use chat_proto::{Message, FETCH_END};
use std::io::Read;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};

/// How long one read waits before the owner loop looks at its queue again.
/// Short, because it bounds how long a tool call waits to be picked up.
const TICK: Duration = Duration::from_millis(100);
/// The cap on unread pushed messages held for `wait`/`read` to report. Beyond
/// it the oldest go: `read` backfills from the server's own history anyway, so
/// the buffer is a wake-up signal, never the record.
const INBOX_MAX: usize = 512;

/// One stored message, as the server's history spells it:
/// `MSG #chan <id> <ts> <nick> :<text>`.
pub struct Row {
    pub id: u64,
    pub ts: u64,
    pub nick: String,
    pub text: String,
}

impl Row {
    /// Parse one history row, or None for any other line.
    pub fn parse(line: &str) -> Option<Row> {
        let mut parts = line.splitn(6, ' ');
        if parts.next()? != "MSG" {
            return None;
        }
        let _chan = parts.next()?;
        let id = parts.next()?.parse().ok()?;
        let ts = parts.next()?.parse().ok()?;
        let nick = parts.next()?.to_string();
        let text = parts.next()?.strip_prefix(':').unwrap_or("").to_string();
        Some(Row { id, ts, nick, text })
    }
}

/// A pushed message, held until a tool asks for it.
pub struct Push {
    pub chan: String,
    pub nick: String,
    pub text: String,
}

/// What the adapter asks the owner thread to do. One variant per tool that
/// needs the connection; the tools that do not (`status`, `discover`,
/// `channels`) never reach here.
#[derive(Clone)]
pub enum Op {
    Join {
        chan: String,
        since: Option<u64>,
    },
    Leave {
        chan: String,
    },
    Send {
        chan: String,
        text: String,
    },
    Read {
        chan: String,
        since: Option<u64>,
        mentions: bool,
    },
    Wait {
        chan: Option<String>,
        mentions: bool,
        timeout: Duration,
    },
    Who {
        chan: String,
    },
}

/// What an operation answers: rows the caller should see, plus the facts a
/// model would otherwise have to infer.
pub struct Answer {
    pub rows: Vec<Row>,
    pub cursor: u64,
    pub members: Vec<String>,
    pub timed_out: bool,
    pub note: Option<String>,
}

impl Answer {
    fn empty() -> Answer {
        Answer {
            rows: Vec::new(),
            cursor: 0,
            members: Vec::new(),
            timed_out: false,
            note: None,
        }
    }
}

type Job = (Op, Sender<Result<Answer, String>>);

/// Why an operation did not answer. `dead` means the connection is gone, so a
/// caller may reopen it; anything else failed on its own terms and reopening
/// would only throw away a working link.
pub struct Failure {
    pub dead: bool,
    pub message: String,
}

impl Failure {
    fn dead(message: &str) -> Failure {
        Failure {
            dead: true,
            message: message.to_string(),
        }
    }
}

/// The adapter's handle on the owner thread.
pub struct Held {
    pub server: String,
    pub nick: String,
    pub state_dir: PathBuf,
    jobs: Sender<Job>,
}

impl Held {
    /// Connect, register, and hand the stream to its owner thread.
    ///
    /// `insecure` is deliberately absent: the TOFU pin is always enforced, so
    /// there is no flag for a model to reach for when a pin mismatch is
    /// inconvenient. A mismatch is a refusal with the reason in it.
    pub fn open(server: &str, nick: &str, state_dir: &std::path::Path) -> Result<Held, String> {
        // message-tags negotiation result unused here: chat-mcp's own push
        // consumer does not yet track a msgid cursor the way chat-client-rs
        // tail does (T135 scoped there only) -- left for a follow-up.
        let (mut tls, _fingerprint, _message_tags) =
            client::connect(server, nick, state_dir, false)?;
        client::wait_for_welcome(&mut tls, nick)?;
        // The registration handshake used a five-second read timeout so a
        // silent server could not hang it. From here the loop wants a tick.
        tls.sock.set_read_timeout(Some(TICK)).ok();
        let (jobs, queue) = channel();
        let owner = Owner {
            tls,
            buf: Vec::new(),
            inbox: Vec::new(),
            nick: nick.to_string(),
            state_dir: state_dir.to_path_buf(),
            mention_seen: std::collections::HashMap::new(),
            closed: false,
        };
        std::thread::spawn(move || owner.run(queue));
        Ok(Held {
            server: server.to_string(),
            nick: nick.to_string(),
            state_dir: state_dir.to_path_buf(),
            jobs,
        })
    }

    /// Submit one operation and wait for its answer. The wait is bounded by
    /// the operation's own deadline plus a margin, so a wedged owner thread
    /// surfaces as an error rather than a hung tool call.
    ///
    /// `dead` separates "this connection is gone" from "this operation did not
    /// work", because only the first is worth reconnecting for. Deciding that
    /// by matching on the error text would be reading tea leaves from our own
    /// prose; the owner thread's disappearance is the fact.
    pub fn submit(&self, op: Op) -> Result<Answer, Failure> {
        let budget = match &op {
            Op::Wait { timeout, .. } => *timeout + Duration::from_secs(10),
            _ => Duration::from_secs(20),
        };
        let (reply, answer) = channel();
        if self.jobs.send((op, reply)).is_err() {
            return Err(Failure::dead(
                "the connection owner stopped; the server closed the link",
            ));
        }
        match answer.recv_timeout(budget) {
            Ok(Ok(answer)) => Ok(answer),
            Ok(Err(message)) => Err(Failure {
                dead: false,
                message,
            }),
            Err(_) => Err(Failure::dead("the connection owner did not answer in time")),
        }
    }
}

/// The owner thread's state: the stream, what it has read but not yet split
/// into lines, and the pushed messages nothing has collected yet.
struct Owner {
    tls: client::Client,
    buf: Vec<u8>,
    inbox: Vec<Push>,
    nick: String,
    state_dir: PathBuf,
    /// How far mention-filtered waiting has reported, per channel. Held in
    /// memory rather than in the session, because a mention read must NOT move
    /// the shared cursor -- the messages it skipped are still unread -- and yet
    /// a second `wait` must not answer with the mention the first one already
    /// returned.
    mention_seen: std::collections::HashMap<String, u64>,
    closed: bool,
}

impl Owner {
    /// Take a job, or read for a tick. Forever, until the adapter drops the
    /// handle or the server closes the link.
    fn run(mut self, queue: Receiver<Job>) {
        loop {
            match queue.try_recv() {
                Ok((op, reply)) => {
                    let answer = self.execute(op);
                    let _ = reply.send(answer);
                }
                Err(TryRecvError::Empty) => {
                    self.next_line(Instant::now() + TICK);
                    if self.closed {
                        return;
                    }
                }
                Err(TryRecvError::Disconnected) => return,
            }
        }
    }

    fn execute(&mut self, op: Op) -> Result<Answer, String> {
        match op {
            Op::Join { chan, since } => self.join(&chan, since),
            Op::Leave { chan } => self.leave(&chan),
            Op::Send { chan, text } => self.send(&chan, &text),
            Op::Read {
                chan,
                since,
                mentions,
            } => self.read(&chan, since, mentions),
            Op::Wait {
                chan,
                mentions,
                timeout,
            } => self.wait(chan.as_deref(), mentions, timeout),
            Op::Who { chan } => self.who(&chan),
        }
    }

    // ---- the operations ---------------------------------------------------

    /// Join, then seed the cursor to the channel's current end so the first
    /// `read` returns what arrives next rather than the whole backlog. An
    /// explicit `since` overrides, which is how a caller asks for history.
    fn join(&mut self, chan: &str, since: Option<u64>) -> Result<Answer, String> {
        self.write(&format!("JOIN {}", chan))?;
        // The 366 that ends the names burst; absent on an older server, so a
        // timeout here is not an error.
        self.await_line(Duration::from_secs(2), |line| line.contains(" 366 "));
        let current = self.last_id(chan)?;
        let seed = since.unwrap_or(current);
        let mut session = client::Session::load(&self.state_dir);
        session.cursors.insert(chan.to_string(), seed);
        let _ = session.save(&self.state_dir);
        Ok(Answer {
            cursor: seed,
            note: Some(format!(
                "joined {}; reads resume after id {} (the channel's end is {})",
                chan, seed, current
            )),
            ..Answer::empty()
        })
    }

    /// Part, and drop the cursor so a later join starts at the end again.
    fn leave(&mut self, chan: &str) -> Result<Answer, String> {
        self.write(&format!("PART {}", chan))?;
        let mut session = client::Session::load(&self.state_dir);
        session.cursors.remove(chan);
        let _ = session.save(&self.state_dir);
        self.inbox.retain(|push| push.chan != chan);
        Ok(Answer {
            note: Some(format!("left {}", chan)),
            ..Answer::empty()
        })
    }

    /// Send one message, as one or more wire lines, and report the id the
    /// server stored it as.
    ///
    /// The id comes from LASTID, not from the echo: an echo carries no id, and
    /// whether a sender is echoed at all is the server's choice.
    fn send(&mut self, chan: &str, text: &str) -> Result<Answer, String> {
        let segments = client::wire_segments(&self.nick, chan, text);
        for segment in &segments {
            self.write(&format!("PRIVMSG {} :{}", chan, segment))?;
        }
        let id = self.last_id(chan)?;
        client::save_cursor(&self.state_dir, chan, id, false);
        Ok(Answer {
            cursor: id,
            note: Some(format!(
                "stored in {} as id {} ({} wire line{})",
                chan,
                id,
                segments.len(),
                if segments.len() == 1 { "" } else { "s" }
            )),
            ..Answer::empty()
        })
    }

    /// The delta: every stored message after the cursor, from the server's own
    /// history, so each row carries its id.
    ///
    /// With no cursor and no `since` this returns nothing and records where
    /// reading starts, rather than dumping a channel's whole history at an
    /// agent that has just arrived. `since: 0` is how history is asked for.
    fn read(&mut self, chan: &str, since: Option<u64>, mentions: bool) -> Result<Answer, String> {
        // A recorded cursor of 0 is not the same as no cursor at all: a join on
        // an empty channel records 0, and reading "after 0" is exactly right
        // there. Treating the two alike skipped the first message a brand-new
        // channel ever received.
        let stored = client::Session::load(&self.state_dir)
            .cursors
            .get(chan)
            .copied();
        let since = match (since, stored) {
            (Some(explicit), _) => explicit,
            (None, None) => {
                let end = self.last_id(chan)?;
                client::save_cursor(&self.state_dir, chan, end, false);
                return Ok(Answer {
                    cursor: end,
                    note: Some(format!(
                        "no cursor for {} yet: reading starts after id {}. Pass since 0 for the backlog.",
                        chan, end
                    )),
                    ..Answer::empty()
                });
            }
            (None, Some(cursor)) => cursor,
        };
        let suffix = if mentions { " mentions" } else { "" };
        self.write(&format!("FETCH {} {}{}", chan, since, suffix))?;
        let lines = self.collect_until(Duration::from_secs(5), |line| line.starts_with(FETCH_END));
        let rows: Vec<Row> = lines.iter().filter_map(|line| Row::parse(line)).collect();
        let top = rows.iter().map(|row| row.id).max().unwrap_or(since);
        // A mention-filtered read must not move the cursor: the messages it
        // skipped were never shown to anyone, and advancing past them would
        // lose them permanently. The CLI makes the same exception.
        if !mentions {
            client::save_cursor(&self.state_dir, chan, top, false);
            self.inbox.retain(|push| push.chan != chan);
        }
        Ok(Answer {
            cursor: if mentions { since } else { top },
            rows,
            ..Answer::empty()
        })
    }

    /// Block until something arrives, then return it as a delta.
    ///
    /// This is the affordance the CLI has no way to offer. The connection is
    /// already subscribed, so the wake-up is the push itself; the rows are then
    /// read back from history so they carry ids and the cursor moves.
    fn wait(
        &mut self,
        chan: Option<&str>,
        mentions: bool,
        timeout: Duration,
    ) -> Result<Answer, String> {
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(woken) = self.pending(chan, mentions) {
                let mut answer = self.deliver(&woken, mentions)?;
                answer.note = Some(format!("woke on a message in {}", woken));
                return Ok(answer);
            }
            if Instant::now() >= deadline {
                return Ok(Answer {
                    timed_out: true,
                    note: Some(format!(
                        "nothing arrived within {}s",
                        timeout.as_secs().max(1)
                    )),
                    ..Answer::empty()
                });
            }
            self.next_line(Instant::now() + TICK);
            if self.closed {
                return Err("the server closed the link".to_string());
            }
        }
    }

    /// Hand back what a wake-up is worth. The pushes it accounts for are
    /// consumed, so a second `wait` does not answer with the same message, and
    /// the rows come from history, so they carry ids.
    ///
    /// `since` is explicit because delegating without one takes the no-cursor
    /// rung, which answers an empty delta on the channel a message just
    /// arrived on.
    fn deliver(&mut self, chan: &str, mentions: bool) -> Result<Answer, String> {
        let cursor = client::Session::load(&self.state_dir).cursor(chan);
        let mention = format!("@{}", self.nick);
        self.inbox
            .retain(|push| !(push.chan == chan && (!mentions || push.text.contains(&mention))));
        let since = if mentions {
            self.mention_seen.get(chan).copied().unwrap_or(cursor)
        } else {
            cursor
        };
        let answer = self.read(chan, Some(since), mentions)?;
        if mentions {
            if let Some(top) = answer.rows.iter().map(|row| row.id).max() {
                self.mention_seen.insert(chan.to_string(), top);
            }
        }
        Ok(answer)
    }

    /// Who is in the channel, from the server's own membership list.
    fn who(&mut self, chan: &str) -> Result<Answer, String> {
        self.write(&format!("NAMES {}", chan))?;
        let lines = self.collect_until(Duration::from_secs(3), |line| line.contains(" 366 "));
        let mut members = Vec::new();
        for line in lines.iter().filter(|line| line.contains(" 353 ")) {
            if let Some((_, names)) = line.rsplit_once(':') {
                members.extend(
                    names
                        .split_whitespace()
                        .map(str::to_string)
                        .filter(|name| !name.is_empty()),
                );
            }
        }
        members.dedup();
        Ok(Answer {
            members,
            ..Answer::empty()
        })
    }

    // ---- the wire ---------------------------------------------------------

    /// The channel with a pushed message matching the filter, if any.
    fn pending(&self, chan: Option<&str>, mentions: bool) -> Option<String> {
        let mention = format!("@{}", self.nick);
        self.inbox
            .iter()
            .find(|push| {
                chan.map(|want| want == push.chan).unwrap_or(true)
                    && (!mentions || push.text.contains(&mention))
            })
            .map(|push| push.chan.clone())
    }

    /// The server's authoritative maximum id for a channel (private numeric
    /// 999), or an error when it does not answer.
    fn last_id(&mut self, chan: &str) -> Result<u64, String> {
        self.write(&format!("LASTID {}", chan))?;
        let reply = self
            .await_line(Duration::from_secs(3), |line| line.contains(" 999 "))
            .ok_or_else(|| format!("no LASTID answer for {} within 3s", chan))?;
        reply
            .split_whitespace()
            .last()
            .and_then(|id| id.parse().ok())
            .ok_or_else(|| format!("unreadable LASTID answer: {}", reply))
    }

    fn write(&mut self, line: &str) -> Result<(), String> {
        client::write_line(&mut self.tls, line)
    }

    /// Read lines until one satisfies `done`, and return it.
    fn await_line<F: Fn(&str) -> bool>(&mut self, timeout: Duration, done: F) -> Option<String> {
        let deadline = Instant::now() + timeout;
        while let Some(line) = self.next_line(deadline) {
            if done(&line) {
                return Some(line);
            }
        }
        None
    }

    /// Read lines until one satisfies `done`, and return everything before it.
    fn collect_until<F: Fn(&str) -> bool>(&mut self, timeout: Duration, done: F) -> Vec<String> {
        let deadline = Instant::now() + timeout;
        let mut lines = Vec::new();
        while let Some(line) = self.next_line(deadline) {
            if done(&line) {
                break;
            }
            lines.push(line);
        }
        lines
    }

    /// The next line that is not a pushed message, or None once the deadline
    /// passes. Pushed messages are routed to the inbox on the way past, so
    /// draining for a reply never loses one.
    fn next_line(&mut self, deadline: Instant) -> Option<String> {
        loop {
            if let Some(line) = self.take_line() {
                return Some(line);
            }
            if self.closed || Instant::now() >= deadline {
                return None;
            }
            let mut chunk = [0u8; 4096];
            match self.tls.read(&mut chunk) {
                Ok(0) => {
                    self.closed = true;
                    return None;
                }
                Ok(n) => self.buf.extend_from_slice(&chunk[..n]),
                Err(error) => match error.kind() {
                    // The tick expired with nothing to read; the partial line
                    // stays in `buf` and the next read continues it.
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut => continue,
                    std::io::ErrorKind::Interrupted => continue,
                    _ => {
                        self.closed = true;
                        return None;
                    }
                },
            }
        }
    }

    /// One complete line out of the read buffer, pushed messages consumed into
    /// the inbox rather than returned.
    fn take_line(&mut self) -> Option<String> {
        loop {
            let end = self.buf.iter().position(|byte| *byte == b'\n')?;
            let line: Vec<u8> = self.buf.drain(..=end).collect();
            let line = String::from_utf8_lossy(&line)
                .trim_end_matches(['\r', '\n'])
                .to_string();
            if line.is_empty() {
                continue;
            }
            if !self.take_push(&line) {
                return Some(line);
            }
        }
    }

    /// Route a pushed PRIVMSG to the inbox. Our own message is not one: the
    /// server may echo it back to us, and waking a `wait` on the message the
    /// same agent just sent would be a loop.
    fn take_push(&mut self, line: &str) -> bool {
        let message = match Message::parse(line) {
            Ok(message) => message,
            Err(_) => return false,
        };
        if message.command != "PRIVMSG" {
            return false;
        }
        let chan = match message.params.first() {
            Some(chan) => chan.clone(),
            None => return false,
        };
        let nick = message
            .prefix
            .as_deref()
            .unwrap_or("")
            .split('!')
            .next()
            .unwrap_or("")
            .to_string();
        if nick == self.nick {
            return true;
        }
        if self.inbox.len() >= INBOX_MAX {
            self.inbox.remove(0);
        }
        self.inbox.push(Push {
            chan,
            nick,
            text: message.trailing.unwrap_or_default(),
        });
        true
    }
}
