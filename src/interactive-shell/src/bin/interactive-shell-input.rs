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
            "text" | "key" | "raw" => {
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
            "shutdown" => {
                if op.is_some() {
                    eprintln!("only one operation is allowed");
                    std::process::exit(2);
                }
                op = Some("shutdown".into());
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
    writeln!(s, "{req}").unwrap();
    s.shutdown(Shutdown::Write).unwrap();
    io::copy(&mut s, &mut io::stdout()).unwrap();
}
