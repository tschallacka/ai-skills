// MODE: DEV
//! W131's own acceptance criteria: proves the shared support module's
//! cross-crate on-demand-build-and-resolve mechanism actually works end to
//! end, run via `cargo test -p chat-server-rs` ALONE -- never preceded by a
//! manual `cargo build -p chat-client-rs` -- before W133/W135/W136 rely on it.

mod support;

use support::{resolve_workspace_binary, spawn_server};

#[test]
fn resolves_and_spawns_chat_server_rs_then_tears_down_cleanly() {
    let server = spawn_server("support-smoke-server", &[("CHAT_ANNOUNCE", "0")]);
    assert!(server.port > 0, "server did not report a usable port");
    // Dropping `server` here kills and reaps the child; a panic inside that
    // drop would fail this test, so reaching this line at all is the "clean
    // exit" the acceptance criteria asks for.
}

#[test]
fn resolves_chat_client_rs_from_inside_chat_server_rs_own_test_binary() {
    // This is the cross-crate case: chat-client-rs is not a dependency of
    // chat-server-rs's Cargo.toml, so `cargo test -p chat-server-rs` alone
    // never builds it -- resolve_workspace_binary must build it on demand.
    let binary = resolve_workspace_binary("chat-client-rs");
    assert!(
        binary.is_file(),
        "resolve_workspace_binary(\"chat-client-rs\") did not resolve to a real file: {}",
        binary.display()
    );
}
