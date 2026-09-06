# Step: 01-step-add-tls-deps

## Ownership

- Goal: `01-irc-server`
- Work unit: `W01`
- Type: `config`

## Change target

- File: `src/chat-server-rs/Cargo.toml`
- Primary symbol or file scope: `[dependencies]`
- Subscope: `N/A`

## Objective

§ 4.1
Add TLS/runtime deps to src/chat-server-rs/Cargo.toml from the src workspace: rustls 0.23 with default-features=false and features=["ring","std","tls12"] (ring backend, NOT the aws-lc-rs default), rustls-pki-types, and ring. Do NOT add rcgen/yasna (not in the offline cache) and do NOT add webpki-roots (TOFU pins the server cert directly). Keep the release profile (opt-level z, lto, strip, panic abort). Wire the crate into the src workspace and depend on the shared chat-proto crate.

## Instructions

§ 5.1
Add rustls (default-features=false, features ring/std/tls12), rustls-pki-types (features std) to src/chat-server-rs/Cargo.toml; keep the chat-proto path dep; move the release profile to the src workspace root.

## Acceptance criteria

§ 6.1
cargo build -p chat-server-rs succeeds; cargo tree shows rustls with the ring provider and no aws-lc-rs.

## Handoff

§ 7.1
W02 can use the TLS types; the server crate builds in the workspace.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
