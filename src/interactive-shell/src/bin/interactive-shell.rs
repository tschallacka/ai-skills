// MODE: DEV
// PACKAGE: PROD
use std::env;
use std::path::PathBuf;
fn main() {
    let a: Vec<String> = env::args().skip(1).collect();
    if a.first().is_some_and(|arg| arg == "--help" || arg == "-h") {
        println!(
            "usage: interactive-shell --socket PATH [OPTIONS] -- COMMAND [ARGUMENTS...]\n\noptions: --cols N --rows N --idle-timeout SECONDS\nThe command runs in a PTY and is controlled through the Unix socket."
        );
        return;
    }
    let mut socket = None;
    let mut cols = 80;
    let mut rows = 24;
    let mut idle = 300;
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
            "--cols" => {
                if i + 1 >= a.len() {
                    eprintln!("--cols requires a value");
                    std::process::exit(2);
                }
                i += 1;
                cols = a.get(i).and_then(|v| v.parse().ok()).unwrap_or_else(|| {
                    eprintln!("invalid cols");
                    std::process::exit(2)
                });
            }
            "--rows" => {
                if i + 1 >= a.len() {
                    eprintln!("--rows requires a value");
                    std::process::exit(2);
                }
                i += 1;
                rows = a.get(i).and_then(|v| v.parse().ok()).unwrap_or_else(|| {
                    eprintln!("invalid rows");
                    std::process::exit(2)
                });
            }
            "--idle-timeout" => {
                if i + 1 >= a.len() {
                    eprintln!("--idle-timeout requires a value");
                    std::process::exit(2);
                }
                i += 1;
                idle = a.get(i).and_then(|v| v.parse().ok()).unwrap_or_else(|| {
                    eprintln!("invalid idle timeout");
                    std::process::exit(2)
                });
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
    let socket = match socket {
        Some(socket) => socket,
        None => {
            eprintln!("--socket is required");
            std::process::exit(2);
        }
    };
    if let Err(e) = interactive_shell_core::run(socket, cols, rows, idle, cmd) {
        eprintln!("interactive-shell: {e}");
        std::process::exit(1)
    }
}
