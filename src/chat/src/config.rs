// MODE: DEV
// PACKAGE: PROD
//! The transport, chosen once by the user and recorded where both the server
//! and the clients can read it.
//!
//! Three outcomes, because there are three genuinely different exposures:
//!
//! | transport | who can reach the bus |
//! |---|---|
//! | `socket` | processes that can open one file: this machine, and only users the socket's mode permits |
//! | `tcp` on `127.0.0.1` | any process on this machine, including other users' |
//! | `tcp` on `0.0.0.0` | anything that can route to this machine |
//!
//! That last row is why the prompt is not three equal-looking options. A nick
//! is self-asserted: `valid_nick` checks the charset and nothing else, and two
//! connections may both claim the same nick. So `0.0.0.0` puts an
//! unauthenticated bus on every interface, where anyone who can reach the host
//! can read every channel and post as anybody. That is a reasonable choice on a
//! trusted network and an accident anywhere else, so it is stated in one plain
//! sentence at the point of choosing.
//!
//! **Precedence, in one line: an explicit flag beats this file, and this file
//! beats the built-in default.** The same order is written into the file's own
//! comment header, because a precedence rule nobody can find is a rule that
//! generates unanswerable bug reports.
//!
//! **Nothing is recorded that a person did not choose.** With no tty there is
//! nobody to ask, so the safest transport is used for that run, the reason goes
//! to stderr, and no file is written — the next interactive run still gets the
//! choice. Writing a default would be worse than not asking, because the file
//! would then look like a decision.
//!
//! **A debug instance never writes it.** `--bind`/`--port` names a debug
//! instance, and a debug instance leaves the default configuration alone; the
//! alternative is that bringing up a debug server on `0.0.0.0` silently
//! re-points every default client.

use std::fmt;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};

/// The port the client helpers already assume, and the fallback when the config
/// says `tcp` and names no port.
pub const DEFAULT_PORT: u16 = 7717;
pub const LOOPBACK: &str = "127.0.0.1";
pub const ANY_INTERFACE: &str = "0.0.0.0";
/// The socket file's name under the home directory.
pub const SOCKET_NAME: &str = "chat.sock";
/// The config file's name under the home directory.
pub const CONFIG_NAME: &str = "config";

/// The longest usable unix socket path.
///
/// `sockaddr_un.sun_path` is a fixed array — 108 bytes on Linux, **104 on
/// macOS** — and the path must fit with its NUL. So this is not a style limit
/// that can be stretched: past it, `bind` fails with a message about `SUN_LEN`
/// that says nothing about what to do. 100 is the smaller platform's cap with
/// margin for the filename, and it is checked when the transport is *chosen* and
/// again when it is *read*, because the failure is otherwise discovered by the
/// user long after the decision, when a client cannot connect.
///
/// This is not hypothetical: `$AI_CHAT_HOME` under a per-session scratch
/// directory blew straight past it during testing.
pub const MAX_SOCKET_PATH: usize = 100;

/// How the bus is reached.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Transport {
    /// A unix domain socket. No port, and nothing on the network.
    Socket(PathBuf),
    Tcp {
        bind: String,
        port: u16,
    },
}

impl Transport {
    /// Is this transport actually usable here? Checked at both ends of the
    /// config's life, so an impossible choice can be neither made nor read.
    pub fn check(&self) -> Result<(), String> {
        match self {
            Transport::Tcp { bind, port } => {
                if bind.is_empty() {
                    return Err("a tcp transport needs a bind address".to_string());
                }
                if *port == 0 {
                    return Err(
                        "port 0 is kernel-assigned, so no client could be pointed at it"
                            .to_string(),
                    );
                }
                Ok(())
            }
            Transport::Socket(path) => {
                if !socket_supported() {
                    return Err(
                        "unix sockets are not available on this platform; use transport=tcp"
                            .to_string(),
                    );
                }
                let len = path.as_os_str().len();
                if len > MAX_SOCKET_PATH {
                    return Err(format!(
                        "the socket path is {len} bytes and the kernel limit is about \
                         {MAX_SOCKET_PATH} (104 on macOS): {}\n\
                         Use a shorter AI_CHAT_HOME or --home, or choose a tcp transport.",
                        path.display()
                    ));
                }
                Ok(())
            }
        }
    }

    /// The address a *client* should dial for this transport.
    ///
    /// A wildcard bind is an address to listen on, not one to connect to:
    /// `0.0.0.0` means "every interface" to `bind` and is not a host. Linux
    /// happens to route a connect to it back to loopback; macOS and Windows are
    /// less obliging, and either way dialling `0.0.0.0` reads as a mistake to
    /// anyone debugging it. So a wildcard is translated to loopback here, in one
    /// place, and `chat/scripts/lib-config.sh` mirrors exactly this rule for the
    /// bash helpers.
    pub fn dial_host(bind: &str) -> &str {
        match bind {
            "0.0.0.0" | "" => LOOPBACK,
            "::" | "[::]" => "::1",
            other => other,
        }
    }

    pub fn loopback() -> Transport {
        Transport::Tcp {
            bind: LOOPBACK.to_string(),
            port: DEFAULT_PORT,
        }
    }

    pub fn any_interface() -> Transport {
        Transport::Tcp {
            bind: ANY_INTERFACE.to_string(),
            port: DEFAULT_PORT,
        }
    }

    pub fn socket_in(home: &Path) -> Transport {
        Transport::Socket(home.join(SOCKET_NAME))
    }

    /// The `key=value` body written to the config file.
    fn to_keys(&self) -> String {
        match self {
            Transport::Socket(p) => format!("transport=socket\nsocket={}\n", p.display()),
            Transport::Tcp { bind, port } => {
                format!("transport=tcp\nbind={bind}\nport={port}\n")
            }
        }
    }
}

impl fmt::Display for Transport {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Transport::Socket(p) => write!(f, "unix socket {}", p.display()),
            Transport::Tcp { bind, port } => write!(f, "tcp {bind}:{port}"),
        }
    }
}

/// Where a transport decision came from. Carried so messages can say *why* the
/// bus is where it is, which is the first question when it is in the wrong
/// place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// An explicit `--bind`/`--port`/`--socket` on this command.
    Flags,
    /// The recorded choice.
    Config,
    /// Chosen by the user just now, and recorded.
    ChosenNow,
    /// No config, no tty: safest transport for this run only, nothing written.
    NoTtyFallback,
}

#[derive(Clone, Debug)]
pub struct Resolved {
    pub transport: Transport,
    pub source: Source,
}

pub fn config_path(home: &Path) -> PathBuf {
    home.join(CONFIG_NAME)
}

/// Read the recorded transport.
///
/// `Ok(None)` means no file, which is the trigger to ask. A file that exists but
/// cannot be understood is an error, never a silent fall back to a default: the
/// user recorded an intention and guessing past it is how a bus ends up
/// somewhere nobody chose.
pub fn load(home: &Path) -> Result<Option<Transport>, String> {
    let path = config_path(home);
    let text = match fs::read_to_string(&path) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(format!("cannot read {}: {e}", path.display())),
    };
    parse(&text, home)
        .map(Some)
        .map_err(|e| format!("{}: {e}", path.display()))
}

/// `key=value`, one per line, `#` comments, blank lines ignored.
///
/// Not JSON, deliberately: `chat-send.sh` and friends have to read this too, and
/// jq is the repository's ceiling for a runtime dependency — a config the shell
/// clients cannot read without jq defeats the point of having one. Unknown keys
/// are ignored so a newer writer's extra key does not break an older reader.
pub fn parse(text: &str, home: &Path) -> Result<Transport, String> {
    let mut transport = None;
    let mut bind = None;
    let mut port = None;
    let mut socket = None;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = match line.split_once('=') {
            Some((k, v)) => (k.trim(), v.trim()),
            None => return Err(format!("line is not key=value: {raw}")),
        };
        match key {
            "transport" => transport = Some(value.to_string()),
            "bind" => bind = Some(value.to_string()),
            "port" => port = Some(value.to_string()),
            "socket" => socket = Some(value.to_string()),
            _ => {}
        }
    }
    let built = match transport.as_deref() {
        Some("socket") => Transport::Socket(match socket {
            Some(s) if !s.is_empty() => PathBuf::from(s),
            _ => home.join(SOCKET_NAME),
        }),
        Some("tcp") => {
            let bind = match bind {
                Some(b) if !b.is_empty() => b,
                _ => return Err("transport=tcp needs a bind= address".to_string()),
            };
            // The documented fallback: tcp with no port means the port every
            // client helper already assumes.
            let port = match port {
                Some(p) if !p.is_empty() => p
                    .parse()
                    .map_err(|_| format!("port={p} is not a port number"))?,
                _ => DEFAULT_PORT,
            };
            Transport::Tcp { bind, port }
        }
        Some(other) => return Err(format!("transport={other} is not one of socket, tcp")),
        None => return Err("no transport= line".to_string()),
    };
    // A recorded transport that cannot work here is an error a person can act
    // on, not a bind failure discovered by whoever next tries to connect.
    built.check()?;
    Ok(built)
}

/// Record the choice, with a header saying when, by whom, and what wins.
pub fn save(home: &Path, transport: &Transport) -> Result<(), String> {
    let path = config_path(home);
    fs::create_dir_all(home).map_err(|e| format!("cannot create {}: {e}", home.display()))?;
    let who = std::env::var("USER").unwrap_or_else(|_| "unknown".to_string());
    let body = format!(
        "# chat transport for this store. Written by `chat` on first use, on \
         behalf of {who}.\n\
         #\n\
         # PRECEDENCE: an explicit --bind/--port/--socket flag beats this file,\n\
         # and this file beats the built-in default. If a client reaches a\n\
         # different server than you expect, a flag is the first thing to look\n\
         # for.\n\
         #\n\
         # Read by the `chat` binary and by the bash helpers, so it is\n\
         # key=value and not JSON: the helpers must not need jq for it.\n\
         #\n\
         # Delete this file to be asked again. Edit it by hand if you prefer;\n\
         # transport is socket or tcp, and tcp with no port= means {DEFAULT_PORT}.\n\
         {}",
        transport.to_keys()
    );
    // Written through a temp file and renamed, so a reader never sees half a
    // config — the clients read this on every call.
    let tmp = path.with_extension("tmp");
    {
        let mut f =
            fs::File::create(&tmp).map_err(|e| format!("cannot write {}: {e}", tmp.display()))?;
        f.write_all(body.as_bytes())
            .map_err(|e| format!("cannot write {}: {e}", tmp.display()))?;
        f.sync_all().map_err(|e| e.to_string())?;
    }
    fs::rename(&tmp, &path).map_err(|e| format!("cannot place {}: {e}", path.display()))
}

/// Is a unix socket even possible here? Rust's `UnixListener` is unix-only, so
/// on Windows the option is not offered and a config naming it is an error a
/// person can act on rather than a panic.
pub const fn socket_supported() -> bool {
    cfg!(unix)
}

/// The transport for a run that has no explicit flags.
///
/// Asks when there is a tty and nothing recorded; otherwise falls back without
/// writing. `ask` is injected so the prompt itself can be tested without a
/// terminal.
pub fn resolve<A>(
    home: &Path,
    is_tty: bool,
    ask: A,
    err: &mut dyn Write,
) -> Result<Resolved, String>
where
    A: FnOnce(&Path, &mut dyn Write) -> Result<Transport, String>,
{
    if let Some(t) = load(home)? {
        return Ok(Resolved {
            transport: t,
            source: Source::Config,
        });
    }
    if !is_tty {
        // Loopback, not the socket, even though the socket exposes less: with
        // nobody watching, the pick must be the one that breaks nothing, and the
        // bash helpers' --host path cannot speak to a unix socket (/dev/tcp has
        // no unix-domain form). Loopback still exposes nothing off the machine.
        let transport = Transport::loopback();
        let _ = writeln!(
            err,
            "chat: no transport chosen for {} and no terminal to ask, so using \
             {transport} for this run only.\n\
             chat: nothing has been recorded — run `chat serve` from a terminal \
             to choose, or write {} by hand.",
            home.display(),
            config_path(home).display()
        );
        return Ok(Resolved {
            transport,
            source: Source::NoTtyFallback,
        });
    }
    let chosen = ask(home, err)?;
    save(home, &chosen)?;
    Ok(Resolved {
        transport: chosen,
        source: Source::ChosenNow,
    })
}

/// The first-run question.
///
/// Not three equal options: the default is named, and the one that exposes the
/// bus to the network says so in a plain sentence rather than in a footnote.
pub fn prompt(home: &Path, err: &mut dyn Write) -> Result<Transport, String> {
    let stdin = io::stdin();
    let mut attempts = 0;
    loop {
        let _ = writeln!(
            err,
            "\nchat has no transport recorded for {}. This is asked once.\n",
            home.display()
        );
        let socket_choice = Transport::socket_in(home);
        let socket_problem = socket_choice.check().err();
        if socket_problem.is_none() {
            let _ = writeln!(
                err,
                "  1) unix socket at {}\n     \
                 Nothing on the network at all, and only users the socket's mode\n     \
                 permits can connect. The bash helpers cannot use it over --host\n     \
                 (there is no unix-domain /dev/tcp), so they would work against the\n     \
                 log instead.",
                home.join(SOCKET_NAME).display()
            );
        } else if let Some(reason) = &socket_problem {
            // Say why it is absent. An option that silently disappears looks
            // like a version difference rather than a path length.
            let _ = writeln!(err, "  1) unix socket — NOT AVAILABLE HERE: {reason}");
        }
        let _ = writeln!(
            err,
            "  2) tcp on {LOOPBACK}:{DEFAULT_PORT}  [default]\n     \
             This machine only. Every client, bash helpers included, works as\n     \
             documented.\n\
             \n  \
             3) tcp on {ANY_INTERFACE}:{DEFAULT_PORT}\n     \
             Every interface. Anyone who can reach this machine can read every\n     \
             channel and post as any nick: the protocol has no authentication and\n     \
             does not enforce unique nicks. Choose this only on a network you\n     \
             trust."
        );
        let _ = write!(err, "\nTransport [2]: ");
        let _ = err.flush();

        let mut answer = String::new();
        match stdin.lock().read_line(&mut answer) {
            // EOF where a tty was expected: do not loop forever asking a
            // terminal that has gone away.
            Ok(0) => return Err("stdin closed before a transport was chosen".to_string()),
            Ok(_) => {}
            Err(e) => return Err(format!("cannot read the answer: {e}")),
        }
        match answer.trim() {
            "" | "2" => return Ok(Transport::loopback()),
            "1" if socket_problem.is_none() => return Ok(socket_choice),
            "1" => {
                let _ = writeln!(
                    err,
                    "\nchat: that option is not available here: {}",
                    socket_problem.clone().unwrap_or_default()
                );
            }
            "3" => return Ok(Transport::any_interface()),
            other => {
                let _ = writeln!(err, "\nchat: {other} is not one of the choices.");
            }
        }
        attempts += 1;
        if attempts >= 3 {
            return Err("no valid transport chosen after three attempts".to_string());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn home(tag: &str) -> PathBuf {
        let mut p = std::env::temp_dir();
        p.push(format!("chat-config-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&p);
        fs::create_dir_all(&p).unwrap();
        p
    }

    fn sink() -> Vec<u8> {
        Vec::new()
    }

    #[test]
    fn no_config_reads_as_nothing_recorded_not_as_an_error() {
        let h = home("absent");
        assert_eq!(load(&h).unwrap(), None);
    }

    #[test]
    fn a_saved_choice_reads_back_identically() {
        for t in [
            Transport::loopback(),
            Transport::any_interface(),
            Transport::Socket(PathBuf::from("/tmp/x/chat.sock")),
        ] {
            let h = home("roundtrip");
            save(&h, &t).unwrap();
            assert_eq!(load(&h).unwrap(), Some(t.clone()), "round trip of {t}");
        }
    }

    #[test]
    fn the_written_file_states_the_precedence_rule() {
        let h = home("header");
        save(&h, &Transport::loopback()).unwrap();
        let text = fs::read_to_string(config_path(&h)).unwrap();
        assert!(
            text.contains("PRECEDENCE"),
            "a config that does not say what wins generates unanswerable bug reports:\n{text}"
        );
        assert!(text.contains("beats this file"), "{text}");
    }

    #[test]
    fn the_written_file_is_key_equals_value_and_not_json() {
        let h = home("format");
        save(&h, &Transport::any_interface()).unwrap();
        let text = fs::read_to_string(config_path(&h)).unwrap();
        assert!(!text.contains('{'), "must be readable without jq:\n{text}");
        let mut seen = Vec::new();
        for line in text.lines() {
            if line.trim().is_empty() || line.starts_with('#') {
                continue;
            }
            let (k, _) = line.split_once('=').expect("every live line is key=value");
            seen.push(k.to_string());
        }
        assert_eq!(seen, vec!["transport", "bind", "port"]);
    }

    #[test]
    fn tcp_with_no_port_falls_back_to_the_port_the_helpers_assume() {
        let h = home("noport");
        let t = parse("transport=tcp\nbind=127.0.0.1\n", &h).unwrap();
        assert_eq!(t, Transport::loopback());
    }

    #[test]
    fn socket_with_no_path_falls_back_to_the_home_directory() {
        let h = home("nopath");
        let t = parse("transport=socket\n", &h).unwrap();
        assert_eq!(t, Transport::Socket(h.join(SOCKET_NAME)));
    }

    #[test]
    fn comments_and_blank_lines_are_ignored() {
        let h = home("comments");
        let t = parse(
            "# a comment\n\n  transport = tcp \n bind = 0.0.0.0 \nport=9\n",
            &h,
        )
        .unwrap();
        assert_eq!(
            t,
            Transport::Tcp {
                bind: "0.0.0.0".to_string(),
                port: 9
            }
        );
    }

    #[test]
    fn an_unknown_key_is_ignored_so_a_newer_writer_does_not_break_an_older_reader() {
        let h = home("unknown");
        let t = parse("transport=tcp\nbind=127.0.0.1\nfuture_option=yes\n", &h).unwrap();
        assert_eq!(t, Transport::loopback());
    }

    /// A file that exists but cannot be understood must NOT fall back to a
    /// default: the user recorded an intention, and guessing past it is how a
    /// bus ends up somewhere nobody chose.
    #[test]
    fn an_unreadable_config_is_an_error_not_a_silent_default() {
        let h = home("garbage");
        fs::write(config_path(&h), "transport=carrier-pigeon\n").unwrap();
        let e = load(&h).unwrap_err();
        assert!(e.contains("not one of"), "unhelpful: {e}");

        fs::write(config_path(&h), "this is not key=value at all\n").unwrap();
        // A line with no '=' is refused rather than skipped.
        assert!(load(&h).is_err());

        fs::write(config_path(&h), "bind=127.0.0.1\n").unwrap();
        assert!(load(&h).unwrap_err().contains("no transport"));
    }

    /// The socket path limit is a kernel array size, not a preference, and it
    /// must be caught when the transport is chosen rather than at bind time —
    /// where it surfaces as a message about SUN_LEN that says nothing about what
    /// to do. A per-session scratch AI_CHAT_HOME really did exceed it.
    #[test]
    fn an_over_long_socket_path_is_refused_with_advice() {
        let long = PathBuf::from(format!("/tmp/{}/chat.sock", "d".repeat(MAX_SOCKET_PATH)));
        let e = Transport::Socket(long).check().unwrap_err();
        assert!(e.contains("kernel limit"), "unhelpful: {e}");
        assert!(
            e.contains("shorter AI_CHAT_HOME") && e.contains("tcp"),
            "must say what to do instead: {e}"
        );
    }

    #[test]
    fn a_workable_socket_path_passes_the_check() {
        assert!(Transport::Socket(PathBuf::from("/tmp/x/chat.sock"))
            .check()
            .is_ok());
    }

    /// Reading is checked as well as writing, because a config can be edited by
    /// hand — the file itself invites it.
    #[test]
    fn a_hand_edited_config_with_an_impossible_socket_is_an_error() {
        let h = home("longsock");
        let long = format!("/tmp/{}/chat.sock", "d".repeat(MAX_SOCKET_PATH));
        fs::write(
            config_path(&h),
            format!("transport=socket\nsocket={long}\n"),
        )
        .unwrap();
        let e = load(&h).unwrap_err();
        assert!(e.contains("kernel limit"), "unhelpful: {e}");
    }

    #[test]
    fn a_recorded_port_zero_is_refused_rather_than_dialled() {
        let h = home("portzero");
        fs::write(config_path(&h), "transport=tcp\nbind=127.0.0.1\nport=0\n").unwrap();
        let e = load(&h).unwrap_err();
        assert!(e.contains("kernel-assigned"), "unhelpful: {e}");
    }

    /// A wildcard bind is an address to listen on, never one to connect to.
    /// Dialling 0.0.0.0 works by accident on Linux and reads as a bug to anyone
    /// debugging it; the bash helpers mirror this same mapping.
    #[test]
    fn a_wildcard_bind_is_dialled_as_loopback() {
        assert_eq!(Transport::dial_host(ANY_INTERFACE), LOOPBACK);
        assert_eq!(Transport::dial_host(""), LOOPBACK);
        assert_eq!(Transport::dial_host("::"), "::1");
        assert_eq!(Transport::dial_host("10.0.0.9"), "10.0.0.9");
        assert_eq!(Transport::dial_host(LOOPBACK), LOOPBACK);
    }

    #[test]
    fn the_config_decides_when_there_are_no_flags() {
        let h = home("resolve-config");
        save(&h, &Transport::any_interface()).unwrap();
        let mut e = sink();
        let r = resolve(
            &h,
            true,
            |_, _| panic!("must not ask when recorded"),
            &mut e,
        )
        .unwrap();
        assert_eq!(r.source, Source::Config);
        assert_eq!(r.transport, Transport::any_interface());
    }

    #[test]
    fn a_first_interactive_run_asks_and_records_exactly_what_was_chosen() {
        let h = home("resolve-ask");
        let mut e = sink();
        let r = resolve(&h, true, |_, _| Ok(Transport::any_interface()), &mut e).unwrap();
        assert_eq!(r.source, Source::ChosenNow);
        assert_eq!(load(&h).unwrap(), Some(Transport::any_interface()));
    }

    #[test]
    fn a_second_run_does_not_ask() {
        let h = home("resolve-twice");
        let mut e = sink();
        resolve(&h, true, |_, _| Ok(Transport::any_interface()), &mut e).unwrap();
        // Asking again would be a bug even if it produced the same answer: the
        // question is stated to the user as being asked once.
        let r = resolve(&h, true, |_, _| panic!("asked a second time"), &mut e).unwrap();
        assert_eq!(r.source, Source::Config);
    }

    /// The one that matters most. A default nobody chose, written to a file that
    /// then looks like a decision, is worse than not asking at all.
    #[test]
    fn a_run_with_no_tty_neither_asks_nor_writes() {
        let h = home("resolve-notty");
        let mut e = sink();
        let r = resolve(
            &h,
            false,
            |_, _| panic!("must not ask without a tty"),
            &mut e,
        )
        .unwrap();
        assert_eq!(r.source, Source::NoTtyFallback);
        assert_eq!(r.transport, Transport::loopback());
        assert_eq!(
            load(&h).unwrap(),
            None,
            "a transport nobody chose must not be recorded"
        );
        let said = String::from_utf8(e).unwrap();
        assert!(said.contains("no terminal"), "must say why: {said}");
        assert!(
            said.contains("nothing has been recorded"),
            "must say nothing was recorded: {said}"
        );
    }

    #[test]
    fn the_no_tty_note_names_the_transport_it_picked() {
        let h = home("notty-names");
        let mut e = sink();
        resolve(&h, false, |_, _| unreachable!(), &mut e).unwrap();
        let said = String::from_utf8(e).unwrap();
        assert!(said.contains("127.0.0.1:7717"), "{said}");
    }
}
