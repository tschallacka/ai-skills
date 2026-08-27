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
//! **The transport is the user's decision**, asked once on first `serve` and
//! recorded in `<home>/config`, which the clients read too. Three outcomes: a
//! unix socket with no port at all, a port on loopback, or a port on every
//! interface. See `config.rs` for why the third is not offered as an equal
//! option, and for why nothing is recorded when there is no terminal to ask.
//!
//! **Only `serve` ever asks.** A client may be in a pipeline, a hook, or a CI
//! step, and a client that can block on a question is a client that can hang a
//! build. With nothing recorded a client acts locally, which needs no server and
//! no decision.
//!
//! **The server's startup contract**, which is the part with teeth:
//!
//! * A second default `serve` detects the live one and declines. It decides
//!   that from an OS lock and a bind, never from `server.pid` — a pid file
//!   outlives its process, and trusting one makes a *fresh* start refuse while
//!   reporting the bus is up, which is the worse direction to fail in.
//! * `--bind`/`--port`, or `--socket`, starts a debug instance alongside it.
//! * A debug instance advertises under `<home>/instances/<endpoint>/`, never
//!   writes the default run files, and **never writes the config** — otherwise
//!   starting a debug server on `0.0.0.0` would silently re-point every default
//!   client.
//! * Therefore bringing up a debug server cannot disturb a bus in use.

mod client;
mod config;
mod instance;
mod net;
mod store;
mod wire;

use client::{ClientError, Range, Target};
use config::{Transport, ANY_INTERFACE, DEFAULT_PORT, LOOPBACK};
use instance::{Busy, Instance};
use std::io::{IsTerminal, Write};
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
        "Usage: chat serve    [--home D] [--bind ADDR --port N | --socket PATH]\n\
         \x20      chat status   [--home D] [--bind ADDR --port N | --socket PATH]\n\
         \x20      chat config   [--home D]\n\
         \x20      chat register #chan [--host H] [--port N] [--socket P] [--local] [--home D]\n\
         \x20      chat send     #chan \"text\" [-n NICK] [...]\n\
         \x20      chat read     #chan [--since N | --last N | --all] [...]\n\
         \x20      chat tail     #chan [--since N] [...]\n\
         \n\
         TRANSPORT\n\
         \x20 Asked once, on the first `serve` with a terminal, and recorded in\n\
         \x20 <home>/config: a unix socket, a port on {LOOPBACK}, or a port on\n\
         \x20 {ANY_INTERFACE}. With no terminal nothing is asked and nothing is\n\
         \x20 recorded. `chat config` prints what is recorded.\n\
         \x20 PRECEDENCE: an explicit flag beats the config; the config beats the\n\
         \x20 built-in default ({LOOPBACK}:{DEFAULT_PORT}).\n\
         \n\
         SERVER\n\
         \x20 With no endpoint flags, serve is THE bus for its home: one at a\n\
         \x20 time. A second serve declines (exit {EX_UNAVAILABLE}) and names the\n\
         \x20 live endpoint.\n\
         \x20 --bind with --port, or --socket, starts a debug instance alongside\n\
         \x20 it. It advertises under <home>/instances/<endpoint>/ - never in the\n\
         \x20 default location, and never in the config - so no client can\n\
         \x20 discover it. Point clients at it explicitly.\n\
         \n\
         CLIENTS\n\
         \x20 --local, or nothing recorded, means no socket at all: send appends\n\
         \x20 under the channel lock, read and tail read the log. That is why they\n\
         \x20 work with no server. Otherwise the recorded transport is used, and\n\
         \x20 an unreachable server falls back to local with a note.\n\
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
    socket: Option<PathBuf>,
    host: Option<String>,
    local: bool,
    nick: String,
    since: Option<u64>,
    last: Option<usize>,
    all: bool,
    positional: Vec<String>,
}

const COMMANDS: [&str; 8] = [
    "serve", "start", "status", "config", "register", "send", "read", "tail",
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
        socket: None,
        host: None,
        local: false,
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
            "--socket" => a.socket = Some(PathBuf::from(it.next().ok_or("--socket needs a path")?)),
            "--local" => a.local = true,
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

/// The endpoint named on the command line, if any.
///
/// Explicit means explicit: `--bind` without `--port` would leave the other half
/// to a default, and a debug instance that silently inherited the default port is
/// exactly the collision this flag pair exists to prevent.
fn explicit_endpoint(a: &Args) -> Result<Option<Transport>, String> {
    if a.socket.is_some() && (a.bind.is_some() || a.port.is_some()) {
        return Err("--socket names an endpoint on its own: drop --bind/--port".into());
    }
    if let Some(p) = &a.socket {
        return Ok(Some(Transport::Socket(p.clone())));
    }
    match (&a.bind, a.port) {
        (Some(b), Some(p)) => Ok(Some(Transport::Tcp {
            bind: b.clone(),
            port: p,
        })),
        (Some(_), None) => Err("--bind needs --port too: an explicit instance names both".into()),
        (None, Some(_)) => Err("--port needs --bind too: an explicit instance names both".into()),
        (None, None) => Ok(None),
    }
}

/// Client-side target, in precedence order.
///
/// Reaching a server at all takes a flag or a recorded transport; there is no
/// probe and no scan, which is what keeps a debug instance unreachable by
/// accident.
fn resolve_target<'a>(a: &'a Args) -> Result<Target<'a>, String> {
    // An explicit --local wins over everything: it is the one way to say "do not
    // talk to a server" when a config says otherwise.
    if a.local {
        return Ok(Target::Local(&a.home));
    }
    if let Some(p) = &a.socket {
        return Ok(Target::Remote(Transport::Socket(p.clone())));
    }
    let recorded = config::load(&a.home)?;
    if let Some(h) = &a.host {
        // A flag beats the config, but only the parts the flag names: --host with
        // no --port takes the recorded port when there is one, so a bus moved off
        // 7717 does not have to be spelled out twice.
        let port = a.port.unwrap_or(match &recorded {
            Some(Transport::Tcp { port, .. }) => *port,
            _ => DEFAULT_PORT,
        });
        return Ok(Target::Remote(Transport::Tcp {
            bind: h.clone(),
            port,
        }));
    }
    if let Some(p) = a.port {
        let bind = match &recorded {
            Some(Transport::Tcp { bind, .. }) => bind.clone(),
            _ => LOOPBACK.to_string(),
        };
        return Ok(Target::Remote(Transport::Tcp { bind, port: p }));
    }
    Ok(match recorded {
        Some(t) => Target::Remote(t),
        // Nothing recorded: act locally rather than asking. A client that can
        // block on a question is a client that can hang a pipeline.
        None => Target::Local(&a.home),
    })
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
        "serve" => serve(&args),
        "status" => status(&args),
        "config" => show_config(&args),
        "register" => with_target(&args, &mut out, |t| {
            channel(&args)
                .map_err(ClientError::Failed)
                .and_then(|c| client::register(&c, t).map(|r| vec![r]))
        }),
        "send" => {
            let text = match args.positional.get(1) {
                Some(t) => t.clone(),
                None => return die_usage("no message given: chat send #chan \"text\""),
            };
            with_target(&args, &mut out, |t| {
                channel(&args)
                    .map_err(ClientError::Failed)
                    .and_then(|c| client::send(&c, &args.nick, &text, t).map(|r| vec![r]))
            })
        }
        "read" => {
            let range = match (args.since, args.last, args.all) {
                // --since wins over --last, as chat-read.sh documents.
                (Some(n), _, _) => Range::Since(n),
                (None, Some(n), _) => Range::Last(n),
                _ => Range::All,
            };
            // Only a local read can be "no such log": a server answers an
            // unknown channel with an empty fetch, and inventing a 66 for that
            // would disagree with chat-read.sh.
            match (resolve_target(&args), channel(&args)) {
                (Ok(Target::Local(home)), Ok(c)) if !log_exists(home, &c) => {
                    eprintln!("chat: no log for {} under {}", c.as_str(), home.display());
                    return ExitCode::from(EX_NOINPUT);
                }
                _ => {}
            }
            with_target(&args, &mut out, |t| {
                channel(&args)
                    .map_err(ClientError::Failed)
                    .and_then(|c| client::read(&c, &range, t))
            })
        }
        "tail" => {
            let target = match resolve_target(&args) {
                Ok(t) => t,
                Err(e) => return die_usage(&e),
            };
            match channel(&args)
                .map_err(ClientError::Failed)
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

/// Run a client operation against the resolved target, falling back to the local
/// log when the server cannot be reached.
///
/// The fallback is announced, never silent. A down server and a working one are
/// different situations even when the outcome looks the same, and a caller that
/// is never told will not notice the bus has been off for a day. Only
/// `Unreachable` is eligible: a protocol refusal means the server heard us and
/// objected, and answering that locally would write what it refused.
fn with_target<F>(args: &Args, out: &mut dyn Write, run: F) -> ExitCode
where
    F: Fn(&Target) -> Result<Vec<String>, ClientError>,
{
    let target = match resolve_target(args) {
        Ok(t) => t,
        Err(e) => return die_usage(&e),
    };
    let result = run(&target);
    match result {
        Ok(rows) => emit(rows, out),
        Err(ClientError::Unreachable(why)) if !matches!(target, Target::Local(_)) => {
            eprintln!(
                "chat: {why}; falling back to the log under {}",
                args.home.display()
            );
            match run(&Target::Local(&args.home)) {
                Ok(rows) => emit(rows, out),
                Err(e) => {
                    eprintln!("chat: {e}");
                    ExitCode::from(EX_UNAVAILABLE)
                }
            }
        }
        Err(e) => {
            eprintln!("chat: {e}");
            ExitCode::from(EX_UNAVAILABLE)
        }
    }
}

fn emit(rows: Vec<String>, out: &mut dyn Write) -> ExitCode {
    for r in rows {
        if writeln!(out, "{r}").is_err() {
            // A closed stdout is `| head`, not a failure.
            return ExitCode::from(EX_OK);
        }
    }
    ExitCode::from(EX_OK)
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

/// `chat config` — print what is recorded, and nothing else.
///
/// Deliberately read-only and non-asking: this is the command a person runs to
/// find out why a client went where it went, and a diagnostic that changes state
/// is not a diagnostic.
fn show_config(args: &Args) -> ExitCode {
    match config::load(&args.home) {
        Ok(Some(t)) => {
            println!("{t}");
            println!("recorded in {}", config::config_path(&args.home).display());
            ExitCode::from(EX_OK)
        }
        Ok(None) => {
            println!(
                "no transport recorded in {}",
                config::config_path(&args.home).display()
            );
            println!("`chat serve` from a terminal will ask; clients act locally until then");
            ExitCode::from(1)
        }
        Err(e) => {
            eprintln!("chat: {e}");
            ExitCode::from(EX_USAGE)
        }
    }
}

/// The transport for this run, where the decision came from, and whether it
/// makes this a debug instance.
///
/// The `Source` is carried and printed rather than discarded, because "why is
/// the bus here" is the first question when it is in the wrong place, and the
/// answer is one of exactly four things.
fn instance_for(args: &Args, may_ask: bool) -> Result<(Instance, config::Source), String> {
    if let Some(t) = explicit_endpoint(args)? {
        return Instance::explicit_with(&args.home, t).map(|i| (i, config::Source::Flags));
    }
    let mut err = std::io::stderr();
    let resolved = if may_ask {
        config::resolve(
            &args.home,
            std::io::stdin().is_terminal(),
            config::prompt,
            &mut err,
        )?
    } else {
        // status must not ask and must not record: it is a question about the
        // world, not a change to it.
        match config::load(&args.home)? {
            Some(t) => config::Resolved {
                transport: t,
                source: config::Source::Config,
            },
            None => config::Resolved {
                transport: Transport::loopback(),
                source: config::Source::NoTtyFallback,
            },
        }
    };
    Ok((
        Instance::default_with(&args.home, resolved.transport),
        resolved.source,
    ))
}

/// How the endpoint was decided, in words a person can act on.
fn why(source: config::Source) -> &'static str {
    match source {
        config::Source::Flags => "endpoint given on the command line",
        config::Source::Config => "transport from the recorded config",
        config::Source::ChosenNow => "transport chosen just now and recorded",
        config::Source::NoTtyFallback => {
            "no transport recorded and no terminal to ask; \
                                         nothing was recorded"
        }
    }
}

fn status(args: &Args) -> ExitCode {
    let (inst, source) = match instance_for(args, false) {
        Ok(i) => i,
        Err(e) => return die_usage(&e),
    };
    // Probing by taking the lock is the only honest answer: if we can take it,
    // nothing lives here, whatever the pid file claims.
    match inst.acquire() {
        Ok(_) => {
            println!(
                "not running ({}); {}",
                inst.run_dir().display(),
                why(source)
            );
            ExitCode::from(1)
        }
        Err(Busy::Held { advertised }) => {
            match advertised {
                Some(a) => println!("running: {a}, run dir {}", inst.run_dir().display()),
                None => println!(
                    "running, but it has not published an endpoint yet ({})",
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

fn serve(args: &Args) -> ExitCode {
    let (inst, source) = match instance_for(args, true) {
        Ok(i) => i,
        Err(e) => return die_usage(&e),
    };
    let home = &args.home;

    let guard = match inst.acquire() {
        Ok(g) => g,
        Err(Busy::Held { advertised }) => {
            // Requirement 1: decline, do not race, and do not take its endpoint.
            match advertised {
                Some(a) => eprintln!(
                    "chat: a server is already running for this home on {a}.\n\
                     To run a debug instance alongside it, name its endpoint:\n  \
                     chat serve --home {} --bind {LOOPBACK} --port <other-port>\n\
                     Clients must then be pointed at that endpoint explicitly.",
                    home.display()
                ),
                None => eprintln!(
                    "chat: a server already holds this home (it has not published an endpoint yet)."
                ),
            }
            return ExitCode::from(EX_UNAVAILABLE);
        }
        Err(Busy::Io(e)) => {
            eprintln!("chat: {e}");
            return ExitCode::from(EX_SOFTWARE);
        }
    };

    // The second witness. The home lock says nobody serves this store; the bind
    // says whether anybody holds this endpoint — which a server on a *different*
    // home could, and no file under this home would show it.
    let listener = match net::Listener::bind(&inst.transport) {
        Ok(l) => l,
        Err(net::BindError::InUse) => {
            eprintln!(
                "chat: {} is already in use, so this bus cannot be served there.\n\
                 Nothing serves this home ({}), so the holder is another process — \
                 most likely a chat server on a different --home.\n\
                 To run alongside it, name a free endpoint:\n  \
                 chat serve --home {} --bind {LOOPBACK} --port <other-port>",
                inst.transport,
                inst.run_dir().display(),
                home.display()
            );
            return ExitCode::from(EX_UNAVAILABLE);
        }
        Err(net::BindError::Other(e)) => {
            eprintln!("chat: cannot bind {}: {e}", inst.transport);
            return ExitCode::from(EX_UNAVAILABLE);
        }
    };

    // Published only now that something is actually listening, so a client never
    // reads an endpoint nothing answers on.
    if let Err(e) = inst.publish(&guard, listener.bound_port()) {
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
        "chat serving: {} ({}), run dir {} [{}]",
        inst.transport,
        if inst.explicit {
            "debug instance"
        } else {
            "default bus"
        },
        inst.run_dir().display(),
        why(source)
    );
    // The wrapper polls for the endpoint and the caller may be reading this
    // line, so do not leave it sitting in a buffer for the life of the process.
    let _ = std::io::stdout().flush();

    // A thread per connection, matching the interpreter tiers. The client count
    // here is agents, not users, so a thread each is cheaper in complexity than
    // an event loop and behaves identically under the load this ever sees.
    loop {
        match listener.accept() {
            Ok(Some(conn)) => {
                let hub = Arc::clone(&hub);
                // A connection that cannot get a thread is dropped rather than
                // taking the server with it.
                let _ = std::thread::Builder::new()
                    .name("chat-conn".to_string())
                    .spawn(move || hub.serve(conn));
            }
            // One connection failed to split; the listener is still good.
            Ok(None) => continue,
            // An accept error is per-connection on every platform that matters
            // (EMFILE, ECONNABORTED), so carry on rather than exiting and
            // dropping every live client.
            Err(e) => {
                eprintln!("chat: accept failed: {e}");
                continue;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    fn home(tag: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("chat-main-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&p);
        std::fs::create_dir_all(&p).unwrap();
        p
    }

    #[test]
    fn serve_with_no_endpoint_flags_is_the_default_bus() {
        let a = parse(args(&["serve", "--home", "/tmp/x"])).unwrap();
        assert_eq!(a.command, "serve");
        assert_eq!(explicit_endpoint(&a).unwrap(), None);
    }

    #[test]
    fn start_is_accepted_as_a_synonym_for_serve() {
        // chat-server.sh's vocabulary; a wrapper's word should not be a trap.
        assert_eq!(parse(args(&["start"])).unwrap().command, "serve");
    }

    #[test]
    fn bind_without_port_is_refused() {
        let a = parse(args(&["serve", "--bind", "127.0.0.1"])).unwrap();
        let e = explicit_endpoint(&a).unwrap_err();
        assert!(e.contains("needs --port"), "unhelpful: {e}");
    }

    #[test]
    fn port_without_bind_is_refused_for_the_server() {
        let a = parse(args(&["serve", "--port", "9999"])).unwrap();
        let e = explicit_endpoint(&a).unwrap_err();
        assert!(e.contains("needs --bind"), "unhelpful: {e}");
    }

    #[test]
    fn both_flags_together_make_an_explicit_tcp_instance() {
        let a = parse(args(&["serve", "--bind", "127.0.0.1", "--port", "9999"])).unwrap();
        assert_eq!(
            explicit_endpoint(&a).unwrap(),
            Some(Transport::Tcp {
                bind: "127.0.0.1".into(),
                port: 9999
            })
        );
    }

    #[test]
    fn socket_alone_makes_an_explicit_socket_instance() {
        let a = parse(args(&["serve", "--socket", "/tmp/d.sock"])).unwrap();
        assert_eq!(
            explicit_endpoint(&a).unwrap(),
            Some(Transport::Socket(PathBuf::from("/tmp/d.sock")))
        );
    }

    /// Two endpoints on one command is a contradiction, and picking one silently
    /// is how a debug server ends up somewhere nobody meant.
    #[test]
    fn socket_together_with_a_port_is_refused() {
        let a = parse(args(&["serve", "--socket", "/tmp/d.sock", "--port", "9"])).unwrap();
        assert!(explicit_endpoint(&a).unwrap_err().contains("on its own"));
    }

    #[test]
    fn a_bad_port_is_a_usage_error_not_a_panic() {
        let e = parse(args(&["serve", "--port", "not-a-port"])).unwrap_err();
        assert!(e.contains("is not a port"), "unhelpful: {e}");
    }

    // ---- the client side of precedence -------------------------------------

    #[test]
    fn with_nothing_recorded_a_client_acts_locally_and_does_not_ask() {
        let h = home("noconfig");
        let a = parse(args(&["read", "#t", "--home", h.to_str().unwrap()])).unwrap();
        assert!(matches!(resolve_target(&a).unwrap(), Target::Local(_)));
    }

    /// The config is what a bare client follows. Without this, recording a
    /// transport would change nothing for the clients that have to use it.
    #[test]
    fn a_bare_client_follows_the_recorded_transport() {
        let h = home("follows");
        config::save(&h, &Transport::any_interface()).unwrap();
        let a = parse(args(&["read", "#t", "--home", h.to_str().unwrap()])).unwrap();
        match resolve_target(&a).unwrap() {
            Target::Remote(t) => assert_eq!(t, Transport::any_interface()),
            Target::Local(_) => panic!("a recorded transport must be used"),
        }

        let h2 = home("follows-sock");
        let sock = Transport::Socket(h2.join("chat.sock"));
        config::save(&h2, &sock).unwrap();
        let b = parse(args(&["read", "#t", "--home", h2.to_str().unwrap()])).unwrap();
        match resolve_target(&b).unwrap() {
            Target::Remote(t) => assert_eq!(t, sock),
            Target::Local(_) => panic!("a recorded socket must be used"),
        }
    }

    /// Precedence, from the client side: the flag wins.
    #[test]
    fn an_explicit_flag_beats_a_config_that_says_otherwise() {
        let h = home("flag-beats");
        config::save(&h, &Transport::any_interface()).unwrap();
        let a = parse(args(&[
            "read",
            "#t",
            "--home",
            h.to_str().unwrap(),
            "--host",
            "10.0.0.9",
            "--port",
            "19999",
        ]))
        .unwrap();
        match resolve_target(&a).unwrap() {
            Target::Remote(Transport::Tcp { bind, port }) => {
                assert_eq!(bind, "10.0.0.9");
                assert_eq!(port, 19999);
            }
            other => panic!(
                "the flag must win, got {other:?}",
                other = match other {
                    Target::Local(_) => "local".to_string(),
                    Target::Remote(t) => t.to_string(),
                }
            ),
        }
    }

    #[test]
    fn local_beats_a_recorded_transport() {
        let h = home("local-beats");
        config::save(&h, &Transport::any_interface()).unwrap();
        let a = parse(args(&[
            "send",
            "#t",
            "x",
            "--home",
            h.to_str().unwrap(),
            "--local",
        ]))
        .unwrap();
        assert!(
            matches!(resolve_target(&a).unwrap(), Target::Local(_)),
            "--local is the one way to say 'do not talk to a server'"
        );
    }

    /// --host with no --port takes the recorded port, so a bus moved off 7717
    /// does not have to be spelled out twice.
    #[test]
    fn host_without_port_inherits_the_recorded_port() {
        let h = home("host-inherit");
        config::save(
            &h,
            &Transport::Tcp {
                bind: LOOPBACK.into(),
                port: 19191,
            },
        )
        .unwrap();
        let a = parse(args(&[
            "read",
            "#t",
            "--home",
            h.to_str().unwrap(),
            "--host",
            "1.2.3.4",
        ]))
        .unwrap();
        match resolve_target(&a).unwrap() {
            Target::Remote(Transport::Tcp { bind, port }) => {
                assert_eq!(bind, "1.2.3.4");
                assert_eq!(port, 19191);
            }
            _ => panic!("expected tcp"),
        }
    }

    #[test]
    fn host_with_nothing_recorded_falls_back_to_the_documented_port() {
        let h = home("host-default");
        let a = parse(args(&[
            "read",
            "#t",
            "--home",
            h.to_str().unwrap(),
            "--host",
            "1.2.3.4",
        ]))
        .unwrap();
        match resolve_target(&a).unwrap() {
            Target::Remote(Transport::Tcp { port, .. }) => assert_eq!(port, DEFAULT_PORT),
            _ => panic!("expected tcp"),
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
