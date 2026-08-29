// MODE: DEV
// PACKAGE: PROD
//! The compiled chat-server rung.
//!
//! Same wire protocol as the interpreter tiers (see chat/runtime/server.py):
//! NICK, JOIN [#chan [since]] with backlog replay, PRIVMSG, FETCH, PING,
//! QUIT; log format `MSG <chan> <id> <ts> <nick> :<text>`; next id is
//! highest+1 (B56); non-MSG log lines are skipped, malformed ones cannot
//! kill the connection (B57); invalid channel and empty text are separate
//! errors (B59); the bound port is printed as bare digits — never through
//! anything that could colour it (B67).
//!
//! The point of compiling: a resident process answers each connection
//! without an interpreter startup, and the lock/id arithmetic runs at native
//! speed. Zero dependencies, like tony-the-pony: nothing about a chat rung
//! justifies a runtime.

use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

fn valid_chan(c: &str) -> bool {
    c.len() > 1
        && c.len() <= 33
        && c.starts_with('#')
        && c[1..].chars().all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
}

fn valid_nick(n: &str) -> bool {
    (1..=32).contains(&n.len()) && n.chars().all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-')
}

struct Hub {
    chan_dir: PathBuf,
    // channel -> highest id seen (the log is the truth; this is the fast path)
    highest: Mutex<HashMap<String, u64>>,
    // connections: writer halves for broadcast
    subs: Mutex<Vec<Arc<Mutex<Option<TcpStream>>>>>,
}

impl Hub {
    fn chan_path(&self, chan: &str) -> PathBuf {
        self.chan_dir.join(format!("{}.log", chan))
    }

    fn scan_highest(&self, chan: &str) -> u64 {
        let path = self.chan_path(chan);
        let mut top = 0u64;
        if let Ok(f) = std::fs::File::open(&path) {
            for line in BufReader::new(f).lines().map_while(Result::ok) {
                let f: Vec<&str> = line.splitn(4, ' ').collect();
                if f.len() >= 4 && f[0] == "MSG" {
                    if let Ok(id) = f[2].parse::<u64>() {
                        if id > top {
                            top = id;
                        }
                    }
                }
            }
        }
        if let Ok(mut h) = self.highest.lock() {
            let e = h.entry(chan.to_string()).or_insert(0);
            if top > *e {
                *e = top;
            }
        }
        top
    }

    fn append(&self, chan: &str, nick: &str, text: &str) -> std::io::Result<(u64, String)> {
        let path = self.chan_path(chan);
        // mkdir-based channel lock, like every other writer: mkdir is atomic.
        let lock = self.chan_dir.join(format!("{}.lock", chan));
        std::fs::create_dir_all(&self.chan_dir)?;
        let mut tries = 0;
        while std::fs::create_dir(&lock).is_err() {
            if lock.join("pid").exists() {
                // corrupt lock: another writer died holding it
                let _ = std::fs::remove_dir_all(&lock);
                continue;
            }
            tries += 1;
            if tries >= 200 {
                return Err(std::io::Error::new(std::io::ErrorKind::Other, "lock timeout"));
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        let result = (|| -> std::io::Result<(u64, String)> {
            let last = self.scan_highest(chan);
            let id = last + 1;
            let ts = SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
            let line = format!("MSG {} {} {} {} :{}\n", chan, id, ts, nick, text.replace('\n', " "));
            let mut f = std::fs::OpenOptions::new().create(true).append(true).open(&path)?;
            f.write_all(line.as_bytes())?;
            if let Ok(mut h) = self.highest.lock() {
                h.insert(chan.to_string(), id);
            }
            Ok((id, line))
        })();
        let _ = std::fs::remove_dir(&lock);
        result
    }

    fn fetch(&self, chan: &str, since: u64) -> Vec<String> {
        let path = self.chan_path(chan);
        let mut out = Vec::new();
        if let Ok(f) = std::fs::File::open(&path) {
            for line in BufReader::new(f).lines().map_while(Result::ok) {
                let f: Vec<&str> = line.splitn(4, ' ').collect();
                if f.len() >= 4 && f[0] == "MSG" && f[1] == chan {
                    if let Ok(id) = f[2].parse::<u64>() {
                        if id >= since {
                            out.push(line);
                        }
                    }
                }
            }
        }
        out
    }

    fn broadcast(&self, line: &str, chan: &str, skip: usize) {
        if let Ok(mut subs) = self.subs.lock() {
            for (i, sub) in subs.iter().enumerate() {
                if i == skip {
                    continue;
                }
                if let Ok(mut guard) = sub.lock() {
                    if let Some(s) = guard.as_mut() {
                        let _ = s.write_all(line.as_bytes());
                        let _ = s.write_all(b"\n");
                        let _ = s.flush();
                    }
                }
            }
        }
        let _ = chan;
    }
}

fn serve(mut stream: TcpStream, hub: Arc<Hub>, slot: Arc<Mutex<Option<TcpStream>>>, idx: usize) {
    let peer_clone = match stream.try_clone() {
        Ok(c) => c,
        Err(_) => return,
    };
    let mut reader = BufReader::new(peer_clone);
    let mut nick = String::new();
    let mut joined: Vec<String> = Vec::new();
    let mut line = String::new();
    loop {
        line.clear();
        if reader.read_line(&mut line).unwrap_or(0) == 0 {
            break;
        }
        let input = line.trim_end_matches(['\r', '\n']);
        let (verb, arg) = match input.split_once(' ') {
            Some((v, a)) => (v, a),
            None => (input, ""),
        };
        let reply = |s: &str| {
            if let Ok(mut guard) = slot.lock() {
                if let Some(w) = guard.as_mut() {
                    let _ = w.write_all(s.as_bytes());
                    let _ = w.write_all(b"\n");
                    let _ = w.flush();
                }
            }
        };
        match verb {
            "NICK" => {
                if !valid_nick(arg) {
                    reply("ERR invalid nick");
                } else {
                    nick = arg.to_string();
                    reply(&format!("OK nick {}", nick));
                }
            }
            "JOIN" => {
                let mut it = arg.split_whitespace();
                let chan = it.next().unwrap_or("");
                let since: u64 = it.next().and_then(|s| s.parse().ok()).unwrap_or(0);
                if !valid_chan(chan) || (it.next().is_some()) {
                    reply("ERR invalid channel");
                    continue;
                }
                if !joined.iter().any(|c| c == chan) {
                    joined.push(chan.to_string());
                }
                reply(&format!("OK join {}", chan));
                // B66: backlog replay after subscribing — duplicate-possible,
                // loss-free.
                for row in hub.fetch(chan, since) {
                    reply(&row);
                }
            }
            "LEAVE" => {
                joined.retain(|c| c != arg);
                reply(&format!("OK leave {}", arg));
            }
            "PRIVMSG" => {
                let (chan, text) = match arg.split_once(" :") {
                    Some((c, t)) => (c, t),
                    None => {
                        reply("ERR usage: PRIVMSG #chan :text");
                        continue;
                    }
                };
                if nick.is_empty() {
                    reply("ERR no nick");
                    continue;
                }
                if !valid_chan(chan) {
                    reply(&format!("ERR invalid channel: {}", chan));
                    continue;
                }
                if text.is_empty() {
                    reply("ERR usage: PRIVMSG #chan :text");
                    continue;
                }
                match hub.append(chan, &nick, text) {
                    Ok((_, line)) => {
                        reply(line.trim_end_matches('\n'));
                        hub.broadcast(&line, chan, idx);
                    }
                    Err(e) => reply(&format!("ERR {}", e)),
                }
            }
            "FETCH" => {
                let (chan, since) = match arg.split_once(' ') {
                    Some((c, s)) => (c, s),
                    None => {
                        reply("ERR usage: FETCH #chan <since-id>");
                        continue;
                    }
                };
                let ok = valid_chan(chan) && since.chars().all(|c| c.is_ascii_digit()) && !since.is_empty();
                if !ok {
                    reply("ERR usage: FETCH #chan <since-id>");
                    continue;
                }
                for row in hub.fetch(chan, since.parse().unwrap_or(0)) {
                    reply(&row);
                }
                reply("OK fetch end");
            }
            "PING" => reply("PONG"),
            "QUIT" => {
                reply("OK bye");
                break;
            }
            _ => reply(&format!("ERR unknown verb {}", verb)),
        }
    }
    if let Ok(mut guard) = slot.lock() {
        *guard = None; // drop the writer; broadcast skips None
    }
}

fn main() {
    let home = std::env::var("AI_CHAT_HOME").unwrap_or_else(|_| {
        eprintln!("chat-server-rs: AI_CHAT_HOME must be set");
        std::process::exit(64);
    });
    let args: Vec<String> = std::env::args().collect();
    let port: u16 = args.get(1).and_then(|p| p.parse().ok()).unwrap_or(0);
    let bind = std::env::var("AI_CHAT_BIND").unwrap_or_else(|_| "127.0.0.1".into());

    let chan_dir = Path::new(&home).join("channels");
    if let Err(e) = std::fs::create_dir_all(&chan_dir) {
        eprintln!("chat-server-rs: cannot create {}: {}", chan_dir.display(), e);
        std::process::exit(66);
    }

    let listener = match TcpListener::bind((bind.as_str(), port)) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("chat-server-rs: cannot bind {}: {}", bind, e);
            std::process::exit(69);
        }
    };
    let actual = listener.local_addr().map(|a| a.port()).unwrap_or(port);
    // The port file is the contract every launcher polls (B63: it must be
    // THIS run's answer); bare digits because a colour-formatted number on
    // another runtime poisoned exactly this line once (B67).
    std::fs::write(Path::new(&home).join("server.port"), format!("{}\n", actual))
        .expect("write server.port");
    println!("{}", actual);

    let hub = Arc::new(Hub {
        chan_dir,
        highest: Mutex::new(HashMap::new()),
        subs: Mutex::new(Vec::new()),
    });

    for stream in listener.incoming() {
        let stream = match stream {
            Ok(s) => s,
            Err(_) => continue,
        };
        // The reader half is cloned before the original moves into the
        // broadcast slot; serve reads its clone and replies through the slot.
        let peer = match stream.try_clone() {
            Ok(p) => p,
            Err(_) => continue,
        };
        let hub = Arc::clone(&hub);
        let mut subs = hub.subs.lock().unwrap();
        let idx = subs.len();
        let slot = Arc::new(Mutex::new(Some(stream)));
        subs.push(Arc::clone(&slot));
        drop(subs);
        std::thread::spawn(move || serve(peer, hub, slot, idx));
    }
}
