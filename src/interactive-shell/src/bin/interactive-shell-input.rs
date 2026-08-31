use serde_json::json;
use std::env;
use std::io::{self, Write};
use std::net::Shutdown;
use std::os::unix::net::UnixStream;
fn main() {
    let a: Vec<String> = env::args().skip(1).collect();
    let mut socket = None;
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
            "text" | "key" | "raw" | "paste" => {
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
            "observe" | "shutdown" => {
                if op.is_some() {
                    eprintln!("only one operation is allowed");
                    std::process::exit(2);
                }
                op = Some(a[i].clone());
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
    let socket = match socket {
        Some(socket) => socket,
        None => {
            eprintln!("--socket is required");
            std::process::exit(2);
        }
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
            Some("text") => req["text"] = v.into(),
            Some("key") => req["key"] = v.into(),
            Some("raw") => req["hex"] = v.into(),
            _ => {}
        }
    }
    let operation = req["op"].as_str().unwrap_or_default().to_owned();
    match operation.as_str() {
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
    io::copy(&mut s, &mut io::stdout()).unwrap();
}
