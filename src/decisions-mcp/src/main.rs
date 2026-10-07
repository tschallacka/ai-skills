// MODE: DEV
// PACKAGE: PROD
//! The MCP transport: one JSON-RPC message per line on stdin, one response per
//! line on stdout. Everything else is in the library, so the protocol can be
//! driven by a test without a process.
//!
//! Unlike `chat-mcp`, no tool here blocks waiting on another party, so every
//! call is answered inline on the one reading thread -- there is nothing a
//! long-lived `wait` could starve.
use std::io::{self, BufRead, Write};

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let message: serde_json::Value = match serde_json::from_str(&line) {
            Ok(message) => message,
            Err(error) => {
                respond(
                    &stdout,
                    serde_json::json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":error.to_string()}}),
                );
                continue;
            }
        };
        respond(&stdout, decisions_mcp::handle(message));
    }
}

fn respond(stdout: &io::Stdout, response: serde_json::Value) {
    if response == serde_json::Value::Null {
        return;
    }
    let mut out = stdout.lock();
    let _ = writeln!(out, "{response}");
    let _ = out.flush();
}
