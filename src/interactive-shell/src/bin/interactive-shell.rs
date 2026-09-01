// MODE: DEV
// PACKAGE: PROD
use std::env;
use std::path::PathBuf;
fn main() {
    let a: Vec<String> = env::args().skip(1).collect();
    if a.first().is_some_and(|arg| arg == "--help" || arg == "-h") {
        println!(
            "usage: interactive-shell [--session ID | --agent ID] [--socket PATH] [OPTIONS] -- COMMAND [ARGUMENTS...]\n\noptions:\n  --session ID             Save and reuse the command, socket, and terminal size\n  --agent ID               Use an agent-keyed saved session\n  --socket PATH            Use an explicit Unix socket (parent must be private)\n  --cols N --rows N        PTY dimensions (default 80x24; prefer smaller when practical)\n  --idle-timeout SECONDS   Stop after inactivity (default 300)\n\nThe command runs in a real PTY. Control it with interactive-shell-input. Start\nwith --session or --agent so later input commands need no socket/configuration\narguments. Without --socket, session sockets are created in a private runtime\ndirectory. The wrapper does not know application keybindings; discover those\nfrom the current screen, built-in help, or a manpage."
        );
        return;
    }
    let mut socket = None;
    let mut session = None;
    let mut agent = None;
    let mut cols = None;
    let mut rows = None;
    let mut idle = None;
    let mut cmd = Vec::new();
    let mut i = 0;
    while i < a.len() {
        match a[i].as_str() {
            "--socket" => {
                if i + 1 >= a.len() {
                    eprintln!("--socket requires a path");
                    std::process::exit(2);
                }
                i += 1;
                socket = Some(PathBuf::from(&a[i]));
            }
            "--session" | "--agent" => {
                if i + 1 >= a.len() {
                    eprintln!("{} requires an id", a[i]);
                    std::process::exit(2);
                }
                i += 1;
                if a[i - 1] == "--session" {
                    session = Some(a[i].clone());
                } else {
                    agent = Some(a[i].clone());
                }
            }
            "--cols" => {
                if i + 1 >= a.len() {
                    eprintln!("--cols requires a value");
                    std::process::exit(2);
                }
                i += 1;
                cols = Some(a.get(i).and_then(|v| v.parse().ok()).unwrap_or_else(|| {
                    eprintln!("invalid cols");
                    std::process::exit(2)
                }));
            }
            "--rows" => {
                if i + 1 >= a.len() {
                    eprintln!("--rows requires a value");
                    std::process::exit(2);
                }
                i += 1;
                rows = Some(a.get(i).and_then(|v| v.parse().ok()).unwrap_or_else(|| {
                    eprintln!("invalid rows");
                    std::process::exit(2)
                }));
            }
            "--idle-timeout" => {
                if i + 1 >= a.len() {
                    eprintln!("--idle-timeout requires a value");
                    std::process::exit(2);
                }
                i += 1;
                idle = Some(a.get(i).and_then(|v| v.parse().ok()).unwrap_or_else(|| {
                    eprintln!("invalid idle timeout");
                    std::process::exit(2)
                }));
            }
            "--" => {
                cmd.extend_from_slice(&a[i + 1..]);
                break;
            }
            x => {
                eprintln!("unknown argument: {x}");
                std::process::exit(2)
            }
        }
        i += 1;
    }
    let session_id = interactive_shell_core::session_identity(session.as_deref(), agent.as_deref());
    let saved = match session_id.as_deref() {
        Some(id) => interactive_shell_core::load_session(id).unwrap_or_else(|error| {
            eprintln!("interactive-shell: {error}");
            std::process::exit(2)
        }),
        None => None,
    };
    if cmd.is_empty() {
        cmd = saved
            .as_ref()
            .map(|s| s.command.clone())
            .unwrap_or_default();
    }
    let cols = cols
        .or_else(|| saved.as_ref().map(|s| s.cols))
        .unwrap_or(80);
    let rows = rows
        .or_else(|| saved.as_ref().map(|s| s.rows))
        .unwrap_or(24);
    let idle = idle
        .or_else(|| saved.as_ref().map(|s| s.idle_timeout))
        .unwrap_or(300);
    let socket = match socket.or_else(|| saved.as_ref().map(|s| s.socket.clone())) {
        Some(socket) => socket,
        None => match session_id.as_deref() {
            Some(id) => interactive_shell_core::session_socket(id).unwrap_or_else(|error| {
                eprintln!("interactive-shell: {error}");
                std::process::exit(2)
            }),
            None => {
                eprintln!("--socket or --session/--agent is required");
                std::process::exit(2);
            }
        },
    };
    if cmd.is_empty() {
        eprintln!("command is required (provide -- COMMAND or a saved session)");
        std::process::exit(2);
    }
    if let Some(id) = session_id {
        let session = interactive_shell_core::Session {
            socket: socket.clone(),
            cols,
            rows,
            idle_timeout: idle,
            command: cmd.clone(),
            agent: interactive_shell_core::session_identity(None, agent.as_deref())
                .or_else(|| saved.as_ref().map(|session| session.agent.clone()))
                .unwrap_or_default(),
        };
        if let Err(error) = interactive_shell_core::save_session(&id, &session) {
            eprintln!("interactive-shell: {error}");
            std::process::exit(2);
        }
    }
    if let Err(e) = interactive_shell_core::run(socket, cols, rows, idle, cmd) {
        eprintln!("interactive-shell: {e}");
        std::process::exit(1)
    }
}
