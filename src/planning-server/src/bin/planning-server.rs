// MODE: DEV
// PACKAGE: PROD
//! planning-server -- the long-running daemon that owns plan document state
//! and serves it, revision-guarded, over a Unix socket. This binary owns
//! only the dispatch loop and transport; `handlers::dispatch` (crate::handlers)
//! implements each of the seven MVP operations.

use planning_server::handlers::dispatch;
use planning_server::protocol::{decode_request, encode_response, Response};
use planning_server::transport::{Listener, Stream};
use std::io::{BufRead, BufReader, Write};

fn handle_client(mut stream: Stream) {
    match stream.authenticate() {
        Ok(true) => {}
        Ok(false) => {
            eprintln!("planning-server: refused a connection that did not present the nonce");
            return;
        }
        Err(error) => {
            eprintln!("planning-server: could not read the connection's nonce: {error}");
            return;
        }
    }
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
    // Listener::bind replaces a stale endpoint left by a prior, no-longer-
    // running server (a live one would already hold the address); a
    // stale-endpoint takeover protocol like ai-text-editor's own is deferred,
    // matching this goal's own recorded narrowing.
    let listener = match Listener::bind(&socket_path) {
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
    loop {
        match listener.accept() {
            Ok(stream) => {
                std::thread::spawn(move || handle_client(stream));
            }
            Err(error) => {
                eprintln!("planning-server: accept error: {error}");
            }
        }
    }
}
