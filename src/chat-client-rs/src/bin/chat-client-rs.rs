// MODE: DEV
// PACKAGE: PROD
//! The `chat-client-rs` command: argv in, the library's CLI entry point out.
//!
//! The client is a library first (`chat_client_rs`) so a second front end can
//! reuse the plumbing that must not be reimplemented -- discovery, the TOFU
//! pin, registration -- rather than re-deriving it. `chat-mcp` is that second
//! front end. This binary is the CLI half and nothing else.
fn main() {
    chat_client_rs::run();
}
