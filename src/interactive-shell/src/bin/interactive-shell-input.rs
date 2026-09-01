// MODE: DEV
// PACKAGE: PROD
use serde_json::json;
use std::env;
use std::io::{self, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
use std::path::PathBuf;
fn main() {
    let a: Vec<String> = env::args().skip(1).collect();
    if a.first().is_some_and(|arg| arg == "--help" || arg == "-h") {
        println!(
            "usage: interactive-shell-input [--session ID | --agent ID] [--socket PATH] OPERATION [ARGUMENTS...]\n\noperations: text, key, combo, raw, paste, observe, wait, mouse, click-id, click-label, click-at, resize, shutdown\nThe socket is read from the session file when --session or --agent is used."
        );
        return;
    }
    let mut socket = None;
    let mut session = None;
    let mut agent = None;
    let mut op = None;
    let mut value = None;
    let mut args = Vec::new();
    let mut i = 0;
    while i < a.len() {
        match a[i].as_str() {
            "--socket" => {
                if i + 1 >= a.len() {
                    eprintln!("--socket requires a path");
                    std::process::exit(2);
                }
                i += 1;
                socket = Some(a[i].clone())
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
            "text" | "paste" => {
                if op.is_some() {
                    eprintln!("only one operation is allowed");
                    std::process::exit(2);
                }
                op = Some(a[i].clone());
                if i + 1 >= a.len() {
                    eprintln!("operation requires a value");
                    std::process::exit(2);
                }
                value = Some(a[i + 1..].join(" "));
                i = a.len() - 1;
            }
            "key" | "raw" => {
                if op.is_some() {
                    eprintln!("only one operation is allowed");
                    std::process::exit(2);
                }
                op = Some(a[i].clone());
                if i + 1 >= a.len() {
                    eprintln!("operation requires a value");
                    std::process::exit(2);
                }
                i += 1;
                value = Some(a[i].clone())
            }
            "combo" => {
                if op.is_some() || i + 1 >= a.len() {
                    eprintln!("combo requires a key and optional modifiers");
                    std::process::exit(2);
                }
                op = Some(a[i].clone());
                args.extend_from_slice(&a[i + 1..]);
                i = a.len() - 1;
            }
            "observe" | "shutdown" => {
                if op.is_some() {
                    eprintln!("only one operation is allowed");
                    std::process::exit(2);
                }
                op = Some(a[i].clone());
            }
            "wait" => {
                if op.is_some() || i + 1 >= a.len() {
                    eprintln!("wait requires text and optional timeout milliseconds");
                    std::process::exit(2);
                }
                op = Some(a[i].clone());
                args.push(a[i + 1].clone());
                i += 1;
                if i + 1 < a.len() {
                    args.push(a[i + 1].clone());
                    i += 1;
                }
            }
            "mouse" => {
                if op.is_some() || i + 4 >= a.len() {
                    eprintln!("mouse requires x y button action");
                    std::process::exit(2);
                }
                op = Some(a[i].clone());
                args.extend_from_slice(&a[i + 1..i + 5]);
                i += 4;
            }
            "click-id" | "click-label" | "click-at" => {
                let count = if a[i] == "click-at" { 3 } else { 2 };
                if op.is_some() || i + count >= a.len() {
                    eprintln!("{} requires its target and button", a[i]);
                    std::process::exit(2);
                }
                op = Some(a[i].clone());
                args.extend_from_slice(&a[i + 1..i + count + 1]);
                i += count;
            }
            "resize" => {
                if op.is_some() || i + 2 >= a.len() {
                    eprintln!("resize requires cols rows");
                    std::process::exit(2);
                }
                op = Some(a[i].clone());
                args.extend_from_slice(&a[i + 1..i + 3]);
                i += 2;
            }
            x => {
                eprintln!("unknown argument: {x}");
                std::process::exit(2)
            }
        }
        i += 1
    }
    let session_id = interactive_shell_core::session_identity(session.as_deref(), agent.as_deref());
    let socket = match socket {
        Some(socket) => PathBuf::from(socket),
        None => match session_id.as_deref() {
            Some(id) => interactive_shell_core::load_session(id)
                .unwrap_or_else(|error| {
                    eprintln!("interactive-shell-input: {error}");
                    std::process::exit(2)
                })
                .map(|s| s.socket)
                .unwrap_or_else(|| {
                    eprintln!("interactive-shell-input: session does not exist");
                    std::process::exit(2)
                }),
            None => {
                eprintln!("--socket or --session/--agent is required");
                std::process::exit(2);
            }
        },
    };
    let op = match op {
        Some(op) => op,
        None => {
            eprintln!("operation required");
            std::process::exit(2);
        }
    };
    let mut s = UnixStream::connect(socket).unwrap_or_else(|error| {
        eprintln!("connect: {error}");
        std::process::exit(1);
    });
    let mut req = json!({"v":1,"op":op});
    if let Some(v) = value {
        match req["op"].as_str() {
            Some("text") | Some("paste") => req["text"] = v.into(),
            Some("key") => req["key"] = v.into(),
            Some("raw") => req["hex"] = v.into(),
            _ => {}
        }
    }
    let operation = req["op"].as_str().unwrap_or_default().to_owned();
    match operation.as_str() {
        "wait" => {
            if args.is_empty() || args.len() > 2 {
                eprintln!("wait requires text and optional timeout milliseconds");
                std::process::exit(2);
            }
            req["contains"] = args[0].clone().into();
            if let Some(timeout) = args.get(1) {
                req["timeout_ms"] = timeout
                    .parse::<u64>()
                    .unwrap_or_else(|_| {
                        eprintln!("invalid wait timeout");
                        std::process::exit(2)
                    })
                    .into();
            }
        }
        "combo" => {
            if args.is_empty() {
                eprintln!("combo requires a key");
                std::process::exit(2);
            }
            req["key"] = args[0].clone().into();
            for modifier in &args[1..] {
                match modifier.to_ascii_lowercase().as_str() {
                    "ctrl" | "control" => req["ctrl"] = true.into(),
                    "alt" | "meta" => req["alt"] = true.into(),
                    "shift" => req["shift"] = true.into(),
                    _ => {
                        eprintln!("unknown combo modifier: {modifier}");
                        std::process::exit(2);
                    }
                }
            }
        }
        "mouse" => {
            if args.len() != 4 {
                eprintln!("mouse requires x y button action");
                std::process::exit(2);
            }
            req["x"] = args[0]
                .parse::<u16>()
                .unwrap_or_else(|_| {
                    eprintln!("invalid mouse x");
                    std::process::exit(2)
                })
                .into();
            req["y"] = args[1]
                .parse::<u16>()
                .unwrap_or_else(|_| {
                    eprintln!("invalid mouse y");
                    std::process::exit(2)
                })
                .into();
            req["button"] = args[2]
                .parse::<u8>()
                .unwrap_or_else(|_| {
                    eprintln!("invalid mouse button");
                    std::process::exit(2)
                })
                .into();
            req["action"] = args[3].clone().into();
        }
        "resize" => {
            if args.len() != 2 {
                eprintln!("resize requires cols rows");
                std::process::exit(2);
            }
            req["cols"] = args[0]
                .parse::<u16>()
                .unwrap_or_else(|_| {
                    eprintln!("invalid cols");
                    std::process::exit(2)
                })
                .into();
            req["rows"] = args[1]
                .parse::<u16>()
                .unwrap_or_else(|_| {
                    eprintln!("invalid rows");
                    std::process::exit(2)
                })
                .into();
        }
        "click-id" | "click-label" => {
            if args.len() != 2 {
                eprintln!("click target requires a value and button");
                std::process::exit(2);
            }
            req["op"] = "click".into();
            if operation == "click-id" {
                req["id"] = args[0].clone().into();
            } else {
                req["label"] = args[0].clone().into();
            }
            req["button"] = args[1]
                .parse::<u8>()
                .unwrap_or_else(|_| {
                    eprintln!("invalid click button");
                    std::process::exit(2)
                })
                .into();
        }
        "click-at" => {
            if args.len() != 3 {
                eprintln!("click-at requires x y button");
                std::process::exit(2);
            }
            req["op"] = "click".into();
            req["x"] = args[0]
                .parse::<u16>()
                .unwrap_or_else(|_| {
                    eprintln!("invalid click x");
                    std::process::exit(2)
                })
                .into();
            req["y"] = args[1]
                .parse::<u16>()
                .unwrap_or_else(|_| {
                    eprintln!("invalid click y");
                    std::process::exit(2)
                })
                .into();
            req["button"] = args[2]
                .parse::<u8>()
                .unwrap_or_else(|_| {
                    eprintln!("invalid click button");
                    std::process::exit(2)
                })
                .into();
        }
        _ => {}
    }
    writeln!(s, "{req}").unwrap();
    s.shutdown(Shutdown::Write).unwrap();
    if let Err(error) = io::copy(&mut s, &mut io::stdout()) {
        if error.kind() != io::ErrorKind::ConnectionReset {
            eprintln!("read response: {error}");
            std::process::exit(1);
        }
    }
}
