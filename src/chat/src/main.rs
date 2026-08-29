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
mod daemon;
mod instance;
mod net;
mod registry;
mod store;
mod wire;

use client::{ClientError, Range, Target};
use config::{Transport, ANY_INTERFACE, DEFAULT_PORT, LOOPBACK};
use instance::{Busy, Instance};
use std::io::{IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::{Duration, Instant};

const EX_OK: u8 = 0;
const EX_USAGE: u8 = 64;
/// Matches `chat-read.sh`: the channel has no log here.
const EX_NOINPUT: u8 = 66;
const EX_UNAVAILABLE: u8 = 69;
const EX_SOFTWARE: u8 = 70;

fn usage() -> String {
    format!(
        "Usage: chat serve    [--home D] [--bind ADDR --port N | --socket PATH] [--no-register]\n\
         \x20      chat stop     [--home D]\n\
         \x20      chat servers\n\
         \x20      chat status   [--home D]\n\
         \x20      chat config   [--home D]\n\
         \x20      chat register #chan [--host H] [--port N] [--socket P] [--local] [--home D]\n\
         \x20      chat send     #chan \"text\" [-n NICK] [...]\n\
         \x20      chat read     #chan [--since N | --last N | --all] [...]\n\
         \x20      chat tail     #chan [--since N] [...]\n\
         \n\
         SERVERS ARE REGISTERED, NOT ASSUMED\n\
         \x20 A server owns the socket; nothing else creates one. It registers in\n\
         \x20 $XDG_RUNTIME_DIR/chat (else <temp>/chat-<uid>, per-uid and 0700), and\n\
         \x20 clients discover it there instead of assuming a port. `chat servers`\n\
         \x20 lists what is registered and whether it answers.\n\
         \x20 An entry is not proof of life: a client decides by connecting, and\n\
         \x20 evicts an entry nothing answers on.\n\
         \n\
         \x20 `chat serve` detaches and keeps running until `chat stop`. A second\n\
         \x20 serve reports the running one rather than starting a rival. A chatter\n\
         \x20 that finds no server STARTS one and adopts it - and says so on stderr.\n\
         \x20 --foreground runs the server in this process (what the detached child\n\
         \x20 does; useful under a supervisor).\n\
         \x20 --no-register serves without claiming the home: no registry entry, no\n\
         \x20 default run files, no config write. That is the debug instance, and it\n\
         \x20 needs an explicit endpoint. Discovery cannot land on it.\n\
         \n\
         TRANSPORT\n\
         \x20 Asked once, on the first `serve` with a terminal, and recorded in\n\
         \x20 <home>/config: a unix socket, a port on {LOOPBACK}, or a port on\n\
         \x20 {ANY_INTERFACE}. With no terminal nothing is asked and nothing is\n\
         \x20 recorded - auto-start always takes that path, so sending a message\n\
         \x20 never records a decision on your behalf.\n\
         \x20 The config is POLICY (how a bus should be exposed); the registry is\n\
         \x20 PRESENCE (where one is). PRECEDENCE: an explicit flag beats both;\n\
         \x20 the registry decides where a client connects; the config decides what\n\
         \x20 a server binds, defaulting to {LOOPBACK}:{DEFAULT_PORT}.\n\
         \n\
         CLIENTS\n\
         \x20 --local means no socket at all: send appends under the channel lock,\n\
         \x20 read and tail read the log. That is why they work with no server.\n\
         \x20 An unreachable server falls back to the log with a note.\n\
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
    foreground: bool,
    no_register: bool,
    nick: String,
    since: Option<u64>,
    last: Option<usize>,
    all: bool,
    positional: Vec<String>,
}

const COMMANDS: [&str; 11] = [
    "serve", "start", "status", "stop", "servers", "config", "register", "send", "read", "tail",
    "attach",
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
        foreground: false,
        no_register: false,
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
            "--foreground" => a.foreground = true,
            "--no-register" => a.no_register = true,
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

/// Start a server for this store, because a chatter needs one and none is
/// registered.
///
/// Michael's rule: when there is no server, there is a need, so start one. The
/// first participant stands the daemon up, it outlives the shell that started it,
/// and everyone after attaches to what is registered.
///
/// **Auto-start never asks the transport question.** The prompt only makes sense
/// on a first interactive `chat serve`; a chatter may be a client in a pipeline,
/// and a client that blocks on a question can hang a build. Worse, answering it
/// here would record a transport decision as a side effect of sending a message —
/// exactly what `config.rs` exists to prevent. So this takes the no-tty path
/// unconditionally: safe default, said out loud, recorded nowhere.
///
/// **And it says what it did.** A client that quietly spawns a daemon is
/// baffling when something later goes wrong, so the line on stderr names what was
/// started and where it registered, which also leaves the auto-start visible in a
/// log after the fact.
fn autostart(args: &Args) -> Option<registry::Entry> {
    let mut err = std::io::stderr();
    let resolved = config::resolve(&args.home, false, |_, _| unreachable!(), &mut err).ok()?;
    let mut flags = endpoint_flags(&resolved.transport);
    flags.push("--home".into());
    flags.push(args.home.display().to_string());
    let log = args.home.join("server.log");
    let _ = std::fs::create_dir_all(&args.home);
    let pid = daemon::spawn_detached(&log, &flags).ok()?;
    // The condition is "a live entry exists", not "my child won": if another
    // chatter started one at the same instant, one of the two takes the home lock
    // and the other exits, and this sees the survivor either way.
    let entry = await_registered(&args.home, Duration::from_secs(10));
    match &entry {
        // Whose server this is matters to the reader. Under a fleet coming up in
        // parallel, every chatter spawns a candidate and exactly one wins the home
        // lock; telling all of them "one was started" would be true of the
        // situation and false of the speaker, and a log full of eight starts for
        // one server is worse than no message at all.
        Some(e) if e.pid == pid => {
            let _ = writeln!(
                err,
                "chat: no server was registered for {}, so one was started: {} (pid {}), \
                 registered in {}",
                args.home.display(),
                e.transport,
                e.pid,
                e.path.display()
            );
        }
        Some(e) => {
            let _ = writeln!(
                err,
                "chat: no server was registered for {}, and another chatter started one at the \
                 same moment; attached to it: {} (pid {})",
                args.home.display(),
                e.transport,
                e.pid
            );
        }
        None => {
            let _ = writeln!(
                err,
                "chat: tried to start a server for {} (pid {}) but it did not register; see {}",
                args.home.display(),
                pid,
                log.display()
            );
        }
    }
    entry
}

/// Client-side target, in precedence order.
///
/// **Discovery replaced convention.** A client used to assume port 7717; it now
/// reads the registry, so the endpoint is a property of the running server rather
/// than a number both sides had to agree on in advance. An explicit flag still
/// wins over discovery, and discovery wins over the built-in default.
///
/// A debug instance is unreachable by accident because it registers nothing:
/// there is no probe and no scan, so the only way to it is being told its
/// address.
fn resolve_target<'a>(a: &'a Args, may_autostart: bool) -> Result<Target<'a>, String> {
    // An explicit --local wins over everything: the one way to say "do not talk
    // to a server" when a server exists.
    if a.local {
        return Ok(Target::Local(&a.home));
    }
    if let Some(p) = &a.socket {
        return Ok(Target::Remote(Transport::Socket(p.clone())));
    }
    // Consulted before the flags are completed, so --host with no --port can
    // inherit the port the server actually registered.
    let live = open_registry()
        .and_then(|r| r.live_for(&a.home))
        .unwrap_or(None);
    let recorded = config::load(&a.home).unwrap_or(None);
    let known_port = || match (&live, &recorded) {
        (Some(e), _) => match &e.transport {
            Transport::Tcp { port, .. } => Some(*port),
            Transport::Socket(_) => None,
        },
        (None, Some(Transport::Tcp { port, .. })) => Some(*port),
        _ => None,
    };
    if let Some(h) = &a.host {
        return Ok(Target::Remote(Transport::Tcp {
            bind: h.clone(),
            port: a.port.or_else(known_port).unwrap_or(DEFAULT_PORT),
        }));
    }
    if let Some(p) = a.port {
        let bind = match (&live, &recorded) {
            (Some(e), _) => match &e.transport {
                Transport::Tcp { bind, .. } => bind.clone(),
                Transport::Socket(_) => LOOPBACK.to_string(),
            },
            (None, Some(Transport::Tcp { bind, .. })) => bind.clone(),
            _ => LOOPBACK.to_string(),
        };
        return Ok(Target::Remote(Transport::Tcp { bind, port: p }));
    }
    if let Some(e) = live {
        return Ok(Target::Remote(e.transport));
    }
    if may_autostart {
        if let Some(e) = autostart(a) {
            return Ok(Target::Remote(e.transport));
        }
        // Auto-start failed. Local mode is still correct and lossless, so the
        // message lands rather than the command failing — but the attempt was
        // already reported above, so this is not a silent fall-through.
        return Ok(Target::Local(&a.home));
    }
    Ok(Target::Local(&a.home))
}

/// The registry, opened once per command. A refusal (someone else's directory,
/// a symlink) is reported by the caller rather than swallowed: it means discovery
/// cannot be trusted here, which the user needs to know.
fn open_registry() -> Result<registry::Registry, String> {
    registry::Registry::open()
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
        "stop" => stop(&args),
        "servers" => servers(&args),
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
            match (resolve_target(&args, true), channel(&args)) {
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
            let target = match resolve_target(&args, true) {
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
    let target = match resolve_target(args, true) {
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

/// The transport for this run, where the decision came from, and whether this
/// process claims the home.
///
/// The `Source` is carried and printed rather than discarded, because "why is
/// the bus here" is the first question when it is in the wrong place, and the
/// answer is one of exactly four things.
fn instance_for(args: &Args, may_ask: bool) -> Result<(Instance, config::Source), String> {
    if let Some(t) = explicit_endpoint(args)? {
        // An explicit endpoint means only "listen here". Whether this server IS
        // the home's bus is a separate question, answered by --no-register - see
        // Instance::registered for why the two used to be conflated and why that
        // could not be reconciled with chat-server.sh's --port.
        let inst = if args.no_register {
            Instance::unregistered(&args.home, t)?
        } else {
            Instance::registered(&args.home, t)
        };
        return Ok((inst, config::Source::Flags));
    }
    if args.no_register {
        return Err(
            "--no-register needs an endpoint: name it with --bind/--port or --socket".into(),
        );
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
        Instance::registered(&args.home, resolved.transport),
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

/// The endpoint flags that reproduce a transport, so a parent can hand its
/// resolved decision to the detached child. The child must never re-resolve:
/// it has no terminal and must never need one.
fn endpoint_flags(t: &Transport) -> Vec<String> {
    match t {
        Transport::Socket(p) => vec!["--socket".into(), p.display().to_string()],
        Transport::Tcp { bind, port } => vec![
            "--bind".into(),
            bind.clone(),
            "--port".into(),
            port.to_string(),
        ],
    }
}

fn status(args: &Args) -> ExitCode {
    // The registry is the answer for a registered bus: it is what a client would
    // consult, so status must consult the same thing or it can disagree with the
    // clients it exists to explain.
    if !args.no_register && explicit_endpoint(args).ok().flatten().is_none() {
        match open_registry().and_then(|r| r.live_for(&args.home)) {
            Ok(Some(e)) => {
                println!(
                    "running: {} (pid {}), registered in {}",
                    e.transport,
                    e.pid,
                    e.path.display()
                );
                return ExitCode::SUCCESS;
            }
            Ok(None) => {
                println!(
                    "not running (nothing registered for {})",
                    args.home.display()
                );
                return ExitCode::from(1);
            }
            Err(e) => eprintln!("chat: {e}"),
        }
    }
    let (inst, source) = match instance_for(args, false) {
        Ok(i) => i,
        Err(e) => return die_usage(&e),
    };
    // For an unregistered instance there is nothing to look up, so fall back to
    // the lock probe: if we can take it, nothing lives here, whatever any file
    // claims.
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

/// `chat servers` — what is registered, and whether it answers.
fn servers(_args: &Args) -> ExitCode {
    let reg = match open_registry() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("chat: {e}");
            return ExitCode::from(EX_UNAVAILABLE);
        }
    };
    let entries = match reg.all() {
        Ok(e) => e,
        Err(e) => {
            eprintln!("chat: {e}");
            return ExitCode::from(EX_UNAVAILABLE);
        }
    };
    println!("registry: {}", reg.dir().display());
    if entries.is_empty() {
        println!("no servers registered");
        return ExitCode::from(1);
    }
    for e in entries {
        // Reported, not evicted: a listing is a diagnostic, and one that deletes
        // what it is describing makes a puzzling situation harder to look at
        // twice. Eviction belongs to whoever actually wanted to connect.
        let live = reg
            .live_for(&e.home)
            .map(|l| l.map(|f| f.path == e.path).unwrap_or(false))
            .unwrap_or(false);
        println!(
            "{} {} home {} pid {}",
            if live { "live " } else { "stale" },
            e.transport,
            e.home.display(),
            e.pid
        );
    }
    ExitCode::from(EX_OK)
}

/// `chat stop` — end the registered server for this home, and tidy up after it.
///
/// The entry is evicted here rather than left for the next client's liveness
/// check. Discovering a corpse works, but tidying up after yourself is not the
/// same thing, and a stale entry between the stop and the next client is a
/// window where `chat servers` lies.
fn stop(args: &Args) -> ExitCode {
    let entry = match open_registry().and_then(|r| r.live_for(&args.home)) {
        Ok(Some(e)) => e,
        Ok(None) => {
            println!(
                "not running (nothing registered for {})",
                args.home.display()
            );
            return ExitCode::from(1);
        }
        Err(e) => {
            eprintln!("chat: {e}");
            return ExitCode::from(EX_UNAVAILABLE);
        }
    };
    if let Err(e) = daemon::terminate(entry.pid) {
        eprintln!("chat: cannot stop pid {}: {e}", entry.pid);
        // The entry is still evicted: whatever that pid is now, it is not a
        // server we can reach, and leaving the entry would send clients at it.
        registry::evict(&entry);
        return ExitCode::from(EX_UNAVAILABLE);
    }
    // Wait for it to actually stop answering, so `stop && serve` cannot race the
    // old server's socket. A bounded wait on a definite condition, not a sleep.
    let deadline = Instant::now() + Duration::from_secs(5);
    while Instant::now() < deadline {
        if net::connect(&entry.transport, false).is_err() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    registry::evict(&entry);
    println!("stopped: {} (pid {})", entry.transport, entry.pid);
    ExitCode::from(EX_OK)
}

/// Wait for a live registered server to appear for this home.
///
/// The condition is "a live entry exists", NOT "my child won the race" - which is
/// what makes the concurrent case fall out for free. Two chatters starting at
/// once both spawn a server; one takes the home lock and registers, the other
/// exits 69; and both waiters see the same single entry appear. The loser never
/// has to know it lost.
fn await_registered(home: &Path, deadline: Duration) -> Option<registry::Entry> {
    let until = Instant::now() + deadline;
    loop {
        if let Ok(Some(e)) = open_registry().and_then(|r| r.live_for(home)) {
            return Some(e);
        }
        if Instant::now() >= until {
            return None;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn serve(args: &Args) -> ExitCode {
    if args.foreground {
        return serve_foreground(args);
    }
    // Requirement: a new participant attaches to an existing server rather than
    // standing one up beside it.
    if !args.no_register {
        match open_registry().and_then(|r| r.live_for(&args.home)) {
            Ok(Some(e)) => {
                println!("already running: {} (pid {})", e.transport, e.pid);
                return ExitCode::from(EX_OK);
            }
            Ok(None) => {}
            Err(e) => {
                eprintln!("chat: {e}");
                return ExitCode::from(EX_UNAVAILABLE);
            }
        }
    }
    // Resolved HERE, in the process that has the terminal. The child is detached
    // and could not ask anything.
    let (inst, source) = match instance_for(args, true) {
        Ok(i) => i,
        Err(e) => return die_usage(&e),
    };
    let mut flags = endpoint_flags(&inst.transport);
    flags.push("--home".into());
    flags.push(args.home.display().to_string());
    if args.no_register {
        flags.push("--no-register".into());
    }
    let log = args.home.join("server.log");
    if let Err(e) = std::fs::create_dir_all(&args.home) {
        eprintln!("chat: cannot create {}: {e}", args.home.display());
        return ExitCode::from(EX_SOFTWARE);
    }
    let pid = match daemon::spawn_detached(&log, &flags) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("chat: {e}");
            return ExitCode::from(EX_SOFTWARE);
        }
    };
    if args.no_register {
        // Nothing to wait for: an unregistered instance publishes nowhere a
        // waiter could look. Its log is the only witness, by design.
        println!(
            "chat serving (detached, unregistered): {} pid {}, log {}",
            inst.transport,
            pid,
            log.display()
        );
        return ExitCode::from(EX_OK);
    }
    match await_registered(&args.home, Duration::from_secs(10)) {
        Some(e) => {
            println!(
                "chat serving (detached): {} pid {} [{}]",
                e.transport,
                e.pid,
                why(source)
            );
            ExitCode::from(EX_OK)
        }
        None => {
            eprintln!(
                "chat: the server did not register within ten seconds. Its log:\n{}",
                std::fs::read_to_string(&log).unwrap_or_default()
            );
            ExitCode::from(EX_UNAVAILABLE)
        }
    }
}

fn serve_foreground(args: &Args) -> ExitCode {
    // Detached from its session by the parent; ignoring SIGHUP as well, because a
    // process can acquire a controlling terminal later.
    daemon::ignore_hangup();
    let (inst, source) = match instance_for(args, false) {
        Ok(i) => i,
        Err(e) => return die_usage(&e),
    };
    let home = &args.home;

    let guard = match inst.acquire() {
        Ok(g) => g,
        Err(Busy::Held { advertised }) => {
            // Losing this race is a normal outcome under auto-start, not a fault:
            // the winner is serving, and whoever is waiting will find its entry.
            match advertised {
                Some(a) => eprintln!(
                    "chat: another server already holds this home on {a}; leaving it alone"
                ),
                None => eprintln!("chat: another server already holds this home; leaving it alone"),
            }
            return ExitCode::from(EX_UNAVAILABLE);
        }
        Err(Busy::Io(e)) => {
            eprintln!("chat: {e}");
            return ExitCode::from(EX_SOFTWARE);
        }
    };

    // The second witness. The home lock says nobody serves this store; the bind
    // says whether anybody holds this endpoint - which a server on a *different*
    // home could, and no file under this home would show it.
    let listener = match net::Listener::bind(&inst.transport) {
        Ok(l) => l,
        Err(net::BindError::InUse) => {
            eprintln!(
                "chat: {} is already in use, so this bus cannot be served there.\n\
                 Nothing serves this home ({}), so the holder is another process - \
                 most likely a chat server on a different --home.",
                inst.transport,
                inst.run_dir().display()
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
    // Registered last, for the same reason and one more: the registry is what
    // clients discover through, so an entry must never exist for a socket that is
    // not yet accepting.
    let mut entry_path = None;
    if inst.registers {
        match open_registry().and_then(|r| r.register(home, &inst.transport)) {
            Ok(p) => entry_path = Some(p),
            Err(e) => {
                eprintln!("chat: cannot register the server: {e}");
                return ExitCode::from(EX_SOFTWARE);
            }
        }
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
        if inst.registers {
            "registered bus"
        } else {
            "debug instance, registers nothing"
        },
        inst.run_dir().display(),
        why(source)
    );
    let _ = std::io::stdout().flush();

    // A thread per connection, matching the interpreter tiers. The client count
    // here is agents, not users, so a thread each is cheaper in complexity than
    // an event loop and behaves identically under the load this ever sees.
    loop {
        match listener.accept() {
            Ok(Some(conn)) => {
                let hub = Arc::clone(&hub);
                let _ = std::thread::Builder::new()
                    .name("chat-conn".to_string())
                    .spawn(move || hub.serve(conn));
            }
            Ok(None) => continue,
            Err(e) => {
                eprintln!("chat: accept failed: {e}");
                let _ = &entry_path;
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
        assert!(matches!(
            resolve_target(&a, false).unwrap(),
            Target::Local(_)
        ));
    }

    /// **The registry subsumed this, and the change is deliberate.** An earlier
    /// version had a bare client follow the recorded transport. It no longer
    /// does: the config is POLICY (how a bus should be exposed, the user's
    /// decision, outliving a reboot) and the registry is PRESENCE (where a bus
    /// actually is, expected to vanish). A recorded transport with nothing
    /// registered means "no server is running", not "connect here" — dialling a
    /// recorded endpoint that nothing published is how a client ends up talking
    /// to whatever else took that port.
    ///
    /// Where a live entry exists, a bare client follows it; that needs a real
    /// server, so it is pinned in chat/tests/test-chat-config.sh rather than
    /// faked here.
    #[test]
    fn a_recorded_transport_does_not_decide_where_a_client_connects() {
        let h = home("policy-not-presence");
        config::save(&h, &Transport::any_interface()).unwrap();
        let a = parse(args(&["read", "#t", "--home", h.to_str().unwrap()])).unwrap();
        assert!(
            matches!(resolve_target(&a, false).unwrap(), Target::Local(_)),
            "with nothing registered, a recorded transport is policy and not a target"
        );
    }

    /// Auto-start is a side effect, so it must never happen on a resolution that
    /// did not ask for it — `chat config`, `chat status` and the tests included.
    #[test]
    fn resolution_without_permission_never_starts_a_server() {
        let h = home("no-autostart");
        let a = parse(args(&["read", "#t", "--home", h.to_str().unwrap()])).unwrap();
        assert!(matches!(
            resolve_target(&a, false).unwrap(),
            Target::Local(_)
        ));
        assert!(
            !h.join("server.log").exists(),
            "nothing should have been launched"
        );
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
        match resolve_target(&a, false).unwrap() {
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
            matches!(resolve_target(&a, false).unwrap(), Target::Local(_)),
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
        match resolve_target(&a, false).unwrap() {
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
        match resolve_target(&a, false).unwrap() {
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
