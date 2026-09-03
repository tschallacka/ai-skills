// MODE: DEV
// PACKAGE: PROD
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
        let response = ai_text_editor_mcp::handle(message);
        if response != serde_json::Value::Null {
            let _ = writeln!(stdout, "{}", response);
            let _ = stdout.flush();
        }
    }
}
