// MODE: DEV
// PACKAGE: PROD
//! planning-server -- the long-running daemon that owns plan document state
//! and serves it, revision-guarded, over a Unix socket. This binary owns
//! only the dispatch loop and transport; `handlers::dispatch` (crate::handlers)
//! implements each of the seven MVP operations.

use planning_server::handlers::dispatch;
use planning_server::protocol::{decode_request, encode_response, Response};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};

fn handle_client(stream: UnixStream) {
    let reader_stream = match stream.try_clone() {
        Ok(cloned) => cloned,
        Err(error) => {
            eprintln!("planning-server: could not clone connection: {error}");
            return;
        }
    };
    let reader = BufReader::new(reader_stream);
    let mut writer = stream;
    for line in reader.lines() {
        let line = match line {
            Ok(line) => line,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }
        let response = match decode_request(&line) {
            Ok(request) => dispatch(request),
            Err(message) => Response::Error { message },
        };
        let encoded = encode_response(&response);
        if writeln!(writer, "{encoded}").is_err() {
            break;
        }
    }
}

fn main() {
    let socket_path = planning_server::endpoint::socket_path();
    if let Some(parent) = socket_path.parent() {
        if let Err(error) = std::fs::create_dir_all(parent) {
            eprintln!(
                "planning-server: cannot create {}: {error}",
                parent.display()
            );
            std::process::exit(74);
        }
    }
    // A stale socket file from a prior, no-longer-running server prevents
    // bind; removing it first is safe because a live server would already
    // hold the address (bind would fail with AddrInUse only while a real
    // listener is active, which this replace-on-start policy accepts as the
    // MVP's own scope -- a stale-endpoint takeover protocol like
    // ai-text-editor's own is deferred, matching this goal's own recorded
    // narrowing).
    let _ = std::fs::remove_file(&socket_path);
    let listener = match UnixListener::bind(&socket_path) {
        Ok(listener) => listener,
        Err(error) => {
            eprintln!(
                "planning-server: cannot bind {}: {error}",
                socket_path.display()
            );
            std::process::exit(74);
        }
    };
    println!("planning-server: listening on {}", socket_path.display());
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                std::thread::spawn(move || handle_client(stream));
            }
            Err(error) => {
                eprintln!("planning-server: accept error: {error}");
            }
        }
    }
}
