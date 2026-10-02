// MODE: DEV
// PACKAGE: PROD
//! The MCP transport: one JSON-RPC message per line on stdin, one response per
//! line on stdout. Everything else is in the library, so the protocol can be
//! driven by a test without a process.
//!
//! A `tools/call` is answered on its own thread. It used to run on this one, so
//! a `wait` that lasted minutes held every other request to the process behind
//! it, including other identities' sends (B363). Responses carry the request's
//! id, so they may leave in any order; the lock on stdout only keeps two of
//! them from interleaving inside a line. Every other method is answered inline,
//! which keeps `initialize` ahead of whatever follows it.
use std::io::{self, BufRead, BufWriter, Stdout, Write};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

type Out = Arc<Mutex<BufWriter<Stdout>>>;

fn respond(out: &Out, response: serde_json::Value) {
    if response == serde_json::Value::Null {
        return;
    }
    let Ok(mut out) = out.lock() else { return };
    let _ = writeln!(out, "{response}");
    let _ = out.flush();
}

fn main() {
    let stdin = io::stdin();
    let out: Out = Arc::new(Mutex::new(BufWriter::new(io::stdout())));
    // A message or timer that an agent asked to be interrupted by leaves as a
    // notification, through the same lock as a response so the two never
    // interleave inside a line. The connection's owner thread is what calls it.
    let notifier_out = Arc::clone(&out);
    chat_mcp::set_notifier(move |notification| respond(&notifier_out, notification));
    let mut in_flight: Vec<JoinHandle<()>> = Vec::new();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let message: serde_json::Value = match serde_json::from_str(&line) {
            Ok(message) => message,
            Err(error) => {
                respond(
                    &out,
                    serde_json::json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":error.to_string()}}),
                );
                continue;
            }
        };
        if message["method"] == "tools/call" {
            let out = Arc::clone(&out);
            in_flight.retain(|worker| !worker.is_finished());
            in_flight.push(std::thread::spawn(move || {
                respond(&out, chat_mcp::handle(message));
            }));
        } else {
            respond(&out, chat_mcp::handle(message));
        }
    }
    // stdin closed: let the calls already in flight answer before exiting, as
    // they did when each one was handled to completion before the next read.
    for worker in in_flight {
        let _ = worker.join();
    }
}
