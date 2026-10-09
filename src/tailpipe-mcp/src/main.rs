// MODE: DEV
// PACKAGE: PROD
//! The MCP transport: one JSON-RPC message per line on stdin, one response
//! per line on stdout. Mirrors src/chat-mcp/src/main.rs's own per-call
//! thread-spawn dispatch for tools/call: `wait` genuinely blocks (up to
//! timeout_seconds), and chat-mcp's own B363 fix is exactly why a blocking
//! tool call must not run on the one thread reading stdin, or it would hold
//! every other request behind it.

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
                respond(&out, tailpipe_mcp::handle(message));
            }));
        } else {
            respond(&out, tailpipe_mcp::handle(message));
        }
    }

    for worker in in_flight {
        let _ = worker.join();
    }
}
