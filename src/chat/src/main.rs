// MODE: DEV
// PACKAGE: PROD
//! `chat` — the ai-skills chat bus as one binary: the server and every client
//! verb, so a host with no bash still has a working client.
//!
//! **Why one binary rather than six.** CODE-STYLE 1b is one binary per crate,
//! and the five helpers are bash — which a Windows agent outside Git Bash does
//! not have. Shipping only a server would have left such a host a bus it could
//! not talk to. Subcommands give both sides in one artifact, one row per target
//! in `chat/binaries.tsv`, and one grammar to document.
//!
//! **The server's startup contract**, which is the part with teeth:
//!
//! * A second default `serve` detects the live one and declines. It decides
//!   that from an OS lock and a bind, never from `server.pid` — a pid file
//!   outlives its process, and trusting one makes a *fresh* start refuse while
//!   reporting the bus is up, which is the worse direction to fail in.
//! * `--bind` and `--port` together start a debug instance alongside it.
//! * A debug instance advertises under `<home>/instances/<bind>_<port>/` and
//!   never writes the default run files, so nothing a default client reads can
//!   point at it. Clients reach it only by being told `--host`/`--port`.
//! * Therefore bringing up a debug server cannot disturb a bus in use.

mod client;
mod instance;
mod store;
mod wire;

use client::{Range, Target};
use instance::{Busy, Instance, DEFAULT_BIND, DEFAULT_PORT};
use std::io::Write;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;

const EX_OK: u8 = 0;
const EX_USAGE: u8 = 64;
/// Matches `chat-read.sh`: the channel has no log here.
const EX_NOINPUT: u8 = 66;
const EX_UNAVAILABLE: u8 = 69;
const EX_SOFTWARE: u8 = 70;

fn usage() -> String {
    format!(
        "Usage: chat serve    [--home D] [--bind ADDR --port N]\n\
         \x20      chat status   [--home D] [--bind ADDR --port N]\n\
         \x20      chat register #chan [--host H] [--port N] [--home D]\n\
         \x20      chat send     #chan \"text\" [-n NICK] [--host H] [--port N] [--home D]\n\
         \x20      chat read     #chan [--since N | --last N | --all] [--host H] [--port N] [--home D]\n\
         \x20      chat tail     #chan [--since N] [--host H] [--port N] [--home D]\n\
         \n\
         SERVER\n\
         \x20 With no --bind/--port, serve is THE bus for its home: one at a time,\n\
         \x20 on {DEFAULT_BIND}:{DEFAULT_PORT}. A second serve declines (exit {EX_UNAVAILABLE})\n\
         \x20 and names the live port.\n\
         \x20 --bind and --port together start a debug instance alongside it. Both\n\
         \x20 are required, the port must be concrete, and it advertises under\n\
         \x20 <home>/instances/<bind>_<port>/ — never in the default location, so\n\
         \x20 no client can discover it. Point clients at it explicitly.\n\
         \n\
         CLIENTS\n\
         \x20 Without --host a client never touches a socket: send appends under the\n\
         \x20 channel lock, read and tail read the log. That is why they work with no\n\
         \x20 server at all. With --host the operation goes to that server, and --port\n\
         \x20 defaults to {DEFAULT_PORT}.\n\
         \n\
         Home defaults to $AI_CHAT_HOME, else $HOME/.ai-chat.\n\
         Exit codes: {EX_USAGE} usage, {EX_NOINPUT} no such log, {EX_UNAVAILABLE} unavailable, {EX_SOFTWARE} internal.\n"
    )
}

#[derive(Debug)]
struct Args {
    command: String,
    home: PathBuf,
    bind: Option<String>,
    port: Option<u16>,
    host: Option<String>,
    nick: String,
    since: Option<u64>,
    last: Option<usize>,
    all: bool,
    positional: Vec<String>,
}

const COMMANDS: [&str; 7] = [
    "serve", "start", "status", "register", "send", "read", "tail",
];

fn parse(argv: Vec<String>) -> Result<Args, String> {
    let default_home = std::env::var_os("AI_CHAT_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".ai-chat")))
        .ok_or("neither AI_CHAT_HOME nor HOME is set, so there is no store to serve")?;

    let mut a = Args {
        command: String::new(),
        home: default_home,
        bind: None,
        port: None,
        host: None,
        // Same default as chat-send.sh, in the same order.
        nick: std::env::var("CHAT_NICK")
            .or_else(|_| std::env::var("USER"))
            .unwrap_or_else(|_| "agent".to_string()),
        since: None,
        last: None,
        all: false,
        positional: Vec::new(),
    };
    let mut it = argv.into_iter();
    while let Some(arg) = it.next() {
        match arg.as_str() {
            c if COMMANDS.contains(&c) && a.command.is_empty() && a.positional.is_empty() => {
                // `start` is accepted because chat-server.sh says start, and a
                // wrapper's vocabulary should not be a trap.
                a.command = if c == "start" {
                    "serve".into()
                } else {
                    c.into()
                };
            }
            "--home" => a.home = PathBuf::from(it.next().ok_or("--home needs a directory")?),
            "--bind" => a.bind = Some(it.next().ok_or("--bind needs an address")?),
            "--host" => a.host = Some(it.next().ok_or("--host needs a hostname")?),
            "-n" | "--nick" => a.nick = it.next().ok_or("-n needs a nick")?,
            "--all" => a.all = true,
            "--port" => {
                let raw = it.next().ok_or("--port needs a number")?;
                a.port = Some(
                    raw.parse()
                        .map_err(|_| format!("--port {raw} is not a port"))?,
                );
            }
            "--since" => {
                let raw = it.next().ok_or("--since needs a message id")?;
                a.since = Some(
                    raw.parse()
                        .map_err(|_| format!("--since {raw} is not a message id"))?,
                );
            }
            "--last" => {
                let raw = it.next().ok_or("--last needs a count")?;
                a.last = Some(
                    raw.parse()
                        .map_err(|_| format!("--last {raw} is not a count"))?,
                );
            }
            "-h" | "--help" => return Err("--help".to_string()),
            other if other.starts_with('-') => return Err(format!("unknown argument: {other}")),
            other => a.positional.push(other.to_string()),
        }
    }
    if a.command.is_empty() {
        return Err("no command given".to_string());
    }
    Ok(a)
}

/// Server-side validation of the endpoint override.
///
/// Explicit means explicit: half an endpoint would leave the other half to a
/// default, and a debug instance that silently inherited the default port is
/// exactly the collision this flag pair exists to prevent.
fn resolve_instance(a: &Args) -> Result<Instance, String> {
    match (&a.bind, a.port) {
        (Some(b), Some(p)) => Instance::explicit_in(&a.home, b, p),
        (Some(_), None) => Err("--bind needs --port too: an explicit instance names both".into()),
        (None, Some(_)) => Err("--port needs --bind too: an explicit instance names both".into()),
        (None, None) => Ok(Instance::default_in(&a.home)),
    }
}

/// Client-side target. `--host` is the only way to reach a server, which is
/// what keeps a debug instance unreachable by accident.
fn resolve_target<'a>(a: &'a Args) -> Target<'a> {
    match &a.host {
        Some(h) => Target::Remote {
            host: h,
            port: a.port.unwrap_or(client::CLIENT_DEFAULT_PORT),
        },
        None => Target::Local(&a.home),
    }
}

fn channel(a: &Args) -> Result<store::Channel, String> {
    let raw = a
        .positional
        .first()
        .ok_or("no channel given: the first argument is #chan")?;
    store::Channel::parse(raw).ok_or_else(|| {
        format!("{raw} is not a channel: # followed by 1 to 32 of lowercase, digit, _ or -")
    })
}

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let args = match parse(argv) {
        Ok(a) => a,
        Err(e) if e == "--help" => {
            print!("{}", usage());
            return ExitCode::SUCCESS;
        }
        Err(e) => return die_usage(&e),
    };

    let out = std::io::stdout();
    let mut out = out.lock();
    match args.command.as_str() {
        "serve" => match resolve_instance(&args) {
            Ok(i) => serve(&i, &args.home),
            Err(e) => die_usage(&e),
        },
        "status" => match resolve_instance(&args) {
            Ok(i) => status(&i),
            Err(e) => die_usage(&e),
        },
        "register" => run(
            channel(&args)
                .and_then(|c| client::register(&c, &resolve_target(&args)).map(|r| vec![r])),
            &mut out,
        ),
        "send" => {
            let text = match args.positional.get(1) {
                Some(t) => t.clone(),
                None => return die_usage("no message given: chat send #chan \"text\""),
            };
            run(
                channel(&args).and_then(|c| {
                    client::send(&c, &args.nick, &text, &resolve_target(&args)).map(|r| vec![r])
                }),
                &mut out,
            )
        }
        "read" => {
            let range = match (args.since, args.last, args.all) {
                // --since wins over --last, as chat-read.sh documents.
                (Some(n), _, _) => Range::Since(n),
                (None, Some(n), _) => Range::Last(n),
                _ => Range::All,
            };
            let target = resolve_target(&args);
            // Only a local read can be "no such log": a server answers an
            // unknown channel with an empty fetch, and inventing a 66 for that
            // would disagree with chat-read.sh.
            if let (Target::Local(home), Ok(c)) = (&target, channel(&args)) {
                if !log_exists(home, &c) {
                    eprintln!("chat: no log for {} under {}", c.as_str(), home.display());
                    return ExitCode::from(EX_NOINPUT);
                }
            }
            run(
                channel(&args).and_then(|c| client::read(&c, &range, &target)),
                &mut out,
            )
        }
        "tail" => {
            let target = resolve_target(&args);
            match channel(&args)
                .and_then(|c| client::tail(&c, args.since.unwrap_or(0), &target, &mut out))
            {
                Ok(()) => ExitCode::from(EX_OK),
                Err(e) => {
                    eprintln!("chat: {e}");
                    ExitCode::from(EX_UNAVAILABLE)
                }
            }
        }
        other => die_usage(&format!("unknown command: {other}")),
    }
}

fn log_exists(home: &Path, chan: &store::Channel) -> bool {
    home.join("channels")
        .join(format!("{}.log", chan.as_str()))
        .exists()
}

fn die_usage(msg: &str) -> ExitCode {
    eprintln!("chat: {msg}\n\n{}", usage());
    ExitCode::from(EX_USAGE)
}

/// Print a client result, or report it and pick the exit code.
fn run(result: Result<Vec<String>, String>, out: &mut dyn Write) -> ExitCode {
    match result {
        Ok(rows) => {
            for r in rows {
                if writeln!(out, "{r}").is_err() {
                    // A closed stdout is `| head`, not a failure.
                    return ExitCode::from(EX_OK);
                }
            }
            ExitCode::from(EX_OK)
        }
        Err(e) => {
            eprintln!("chat: {e}");
            ExitCode::from(EX_UNAVAILABLE)
        }
    }
}

fn status(inst: &Instance) -> ExitCode {
    // Probing by taking the lock is the only honest answer: if we can take it,
    // nothing lives here, whatever the pid file claims.
    match inst.acquire() {
        Ok(_) => {
            println!("not running ({})", inst.run_dir().display());
            ExitCode::from(1)
        }
        Err(Busy::Held { advertised }) => {
            match advertised {
                Some(p) => println!(
                    "running: port {p}, bind {}, run dir {}",
                    inst.bind,
                    inst.run_dir().display()
                ),
                None => println!(
                    "running, but it has not published a port yet ({})",
                    inst.run_dir().display()
                ),
            }
            ExitCode::SUCCESS
        }
        Err(Busy::Io(e)) => {
            eprintln!("chat: {e}");
            ExitCode::from(EX_SOFTWARE)
        }
    }
}

fn serve(inst: &Instance, home: &Path) -> ExitCode {
    let guard = match inst.acquire() {
        Ok(g) => g,
        Err(Busy::Held { advertised }) => {
            // Requirement 1: decline, do not race, and do not take its endpoint.
            match advertised {
                Some(p) => eprintln!(
                    "chat: a server is already running for this home on port {p}.\n\
                     To run a debug instance alongside it, name its endpoint:\n  \
                     chat serve --home {} --bind {DEFAULT_BIND} --port <other-port>\n\
                     Clients must then be pointed at that port explicitly.",
                    home.display()
                ),
                None => eprintln!(
                    "chat: a server already holds this home (it has not published a port yet)."
                ),
            }
            return ExitCode::from(EX_UNAVAILABLE);
        }
        Err(Busy::Io(e)) => {
            eprintln!("chat: {e}");
            return ExitCode::from(EX_SOFTWARE);
        }
    };

    // The second witness. The home lock says nobody serves this store; the
    // bind says whether anybody holds this endpoint — which a server on a
    // *different* home could, and no file under this home would show it.
    let listener = match TcpListener::bind((inst.bind.as_str(), inst.port)) {
        Ok(l) => l,
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
            eprintln!(
                "chat: {}:{} is already in use, so this bus cannot be served there.\n\
                 Nothing serves this home ({}), so the holder is another process — \
                 most likely a chat server on a different --home.\n\
                 To run alongside it, name a free endpoint:\n  \
                 chat serve --home {} --bind {} --port <other-port>\n\
                 Clients must then be pointed at that port explicitly.",
                inst.bind,
                inst.port,
                inst.run_dir().display(),
                home.display(),
                inst.bind
            );
            return ExitCode::from(EX_UNAVAILABLE);
        }
        Err(e) => {
            eprintln!("chat: cannot bind {}:{}: {e}", inst.bind, inst.port);
            return ExitCode::from(EX_UNAVAILABLE);
        }
    };
    let bound = match listener.local_addr() {
        Ok(a) => a.port(),
        Err(e) => {
            eprintln!("chat: bound but cannot read the local address: {e}");
            return ExitCode::from(EX_SOFTWARE);
        }
    };

    // Published only now that something is actually listening, so a client
    // never reads a port nothing answers on.
    if let Err(e) = inst.publish(&guard, bound) {
        eprintln!("chat: cannot publish the endpoint: {e}");
        return ExitCode::from(EX_SOFTWARE);
    }

    let store = match store::Store::new(home) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("chat: cannot open the store under {}: {e}", home.display());
            return ExitCode::from(EX_SOFTWARE);
        }
    };
    let hub = Arc::new(wire::Hub::new(store));

    println!(
        "chat serving: {}:{} ({}), run dir {}",
        inst.bind,
        bound,
        if inst.explicit {
            "debug instance"
        } else {
            "default bus"
        },
        inst.run_dir().display()
    );
    // The wrapper polls server.port and the caller may be reading this line, so
    // do not leave it sitting in a buffer for the life of the process.
    let _ = std::io::stdout().flush();

    // A thread per connection, matching the interpreter tiers. The client count
    // here is agents, not users, so a thread each is cheaper in complexity than
    // an event loop and behaves identically under the load this ever sees.
    for stream in listener.incoming() {
        match stream {
            Ok(s) => {
                let hub = Arc::clone(&hub);
                // A connection that cannot get a thread is dropped rather than
                // taking the server with it.
                let _ = std::thread::Builder::new()
                    .name("chat-conn".to_string())
                    .spawn(move || hub.serve(s));
            }
            // An accept error is per-connection on every platform that matters
            // (EMFILE, ECONNABORTED); the listener is still good, so carry on
            // rather than exiting and dropping every live client.
            Err(e) => {
                eprintln!("chat: accept failed: {e}");
                continue;
            }
        }
    }
    // `incoming()` never ends, so reaching here is a platform failure.
    drop(guard);
    ExitCode::from(EX_SOFTWARE)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn serve_with_no_endpoint_is_the_default_bus() {
        let a = parse(args(&["serve", "--home", "/tmp/x"])).unwrap();
        assert_eq!(a.command, "serve");
        let i = resolve_instance(&a).unwrap();
        assert!(!i.explicit);
        assert_eq!(i.port, DEFAULT_PORT);
    }

    #[test]
    fn start_is_accepted_as_a_synonym_for_serve() {
        // chat-server.sh's vocabulary; a wrapper's word should not be a trap.
        assert_eq!(parse(args(&["start"])).unwrap().command, "serve");
    }

    #[test]
    fn bind_without_port_is_refused() {
        let a = parse(args(&["serve", "--home", "/tmp/x", "--bind", "127.0.0.1"])).unwrap();
        let e = resolve_instance(&a).unwrap_err();
        assert!(e.contains("needs --port"), "unhelpful: {e}");
    }

    #[test]
    fn port_without_bind_is_refused_for_the_server() {
        let a = parse(args(&["serve", "--home", "/tmp/x", "--port", "9999"])).unwrap();
        let e = resolve_instance(&a).unwrap_err();
        assert!(e.contains("needs --bind"), "unhelpful: {e}");
    }

    #[test]
    fn both_flags_together_make_an_explicit_instance() {
        let a = parse(args(&[
            "serve",
            "--home",
            "/tmp/x",
            "--bind",
            "127.0.0.1",
            "--port",
            "9999",
        ]))
        .unwrap();
        let i = resolve_instance(&a).unwrap();
        assert!(i.explicit);
        assert_eq!(i.port, 9999);
    }

    #[test]
    fn a_bad_port_is_a_usage_error_not_a_panic() {
        let e = parse(args(&["serve", "--port", "not-a-port"])).unwrap_err();
        assert!(e.contains("is not a port"), "unhelpful: {e}");
    }

    #[test]
    fn a_client_without_host_acts_locally() {
        let a = parse(args(&["read", "#t", "--home", "/tmp/x"])).unwrap();
        assert!(matches!(resolve_target(&a), Target::Local(_)));
    }

    /// Requirement 3, from the client side: reaching a server at all takes
    /// --host, and reaching a debug instance takes --host and --port. There is
    /// no path from a bare client to an endpoint nobody named.
    #[test]
    fn a_client_reaches_a_server_only_when_told_to() {
        let a = parse(args(&["read", "#t", "--host", "127.0.0.1"])).unwrap();
        match resolve_target(&a) {
            Target::Remote { host, port } => {
                assert_eq!(host, "127.0.0.1");
                assert_eq!(port, DEFAULT_PORT, "--host alone means the default bus");
            }
            Target::Local(_) => panic!("--host should mean remote"),
        }
        let b = parse(args(&[
            "read",
            "#t",
            "--host",
            "127.0.0.1",
            "--port",
            "19999",
        ]))
        .unwrap();
        match resolve_target(&b) {
            Target::Remote { port, .. } => assert_eq!(port, 19999),
            Target::Local(_) => panic!("--host should mean remote"),
        }
    }

    #[test]
    fn the_channel_argument_is_validated_before_use() {
        let a = parse(args(&["send", "NotAChannel", "hi"])).unwrap();
        let e = channel(&a).unwrap_err();
        assert!(e.contains("is not a channel"), "unhelpful: {e}");
    }

    #[test]
    fn a_missing_channel_is_a_usage_error() {
        let a = parse(args(&["send"])).unwrap();
        assert!(channel(&a).unwrap_err().contains("no channel given"));
    }

    #[test]
    fn no_command_is_a_usage_error_rather_than_a_silent_default() {
        // A bare `chat` used to mean "start the server". Guessing a verb that
        // takes over a port is not a good default.
        assert!(parse(args(&["--home", "/tmp/x"]))
            .unwrap_err()
            .contains("no command"));
    }

    #[test]
    fn a_channel_is_not_mistaken_for_a_command() {
        // `#read` is a legal channel name, and the command slot is already
        // filled, so it must stay a positional.
        let a = parse(args(&["send", "#read", "hi"])).unwrap();
        assert_eq!(a.command, "send");
        assert_eq!(a.positional, vec!["#read", "hi"]);
    }

    #[test]
    fn the_nick_default_follows_chat_send_sh() {
        let a = parse(args(&["send", "#t", "hi"])).unwrap();
        assert!(!a.nick.is_empty(), "a nick must always resolve");
        let b = parse(args(&["send", "#t", "hi", "-n", "codex"])).unwrap();
        assert_eq!(b.nick, "codex");
    }
}
