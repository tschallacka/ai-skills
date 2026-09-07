// MODE: DEV
// PACKAGE: PROD
//! The MCP transport: one JSON-RPC message per line on stdin, one response per
//! line on stdout. Everything else is in the library, so the protocol can be
//! driven by a test without a process.
use std::io::{self, BufRead, Write};

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::BufWriter::new(io::stdout());
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let message = match serde_json::from_str(&line) {
            Ok(message) => message,
            Err(error) => {
                let _ = writeln!(
                    stdout,
                    "{}",
                    serde_json::json!({"jsonrpc":"2.0","id":null,"error":{"code":-32700,"message":error.to_string()}})
                );
                let _ = stdout.flush();
                continue;
            }
        };
        let response = chat_mcp::handle(message);
        if response != serde_json::Value::Null {
            let _ = writeln!(stdout, "{}", response);
            let _ = stdout.flush();
        }
    }
}
