# Step: 01-step-crate-scaffold

## Ownership

- Goal: `02-irc-client`
- Work unit: `W10`
- Type: `config`

## Change target

- File: `src/chat-client-rs/Cargo.toml`
- Primary symbol or file scope: `[package]+[dependencies]`
- Subscope: `N/A`

## Objective

§ 4.1
Create the client crate scaffold: src/chat-client-rs/{Cargo.toml,rust-toolchain.toml(src workspace 1.97, rustfmt+clippy),src/main.rs}, MODE: DEV / PACKAGE: PROD headers, release profile opt-level z/lto/strip/panic abort. Dependencies: rustls 0.23 (default-features=false, features=["ring","std","tls12"]), rustls-pki-types, ring, and the shared src/chat-proto crate. NOT webpki-roots (TOFU pins the cert).

## Instructions

§ 5.1
Create src/chat-client-rs/{Cargo.toml,rust-toolchain.toml(1.97,rustfmt+clippy),src/main.rs}, MODE DEV/PACKAGE PROD; deps rustls(default-features=false, features ring/std/tls12), rustls-pki-types(std), chat-proto. Add chat-client-rs to the src workspace members.

## Acceptance criteria

§ 6.1
cargo build -p chat-client-rs succeeds; the crate is a workspace member; clippy -D warnings clean.

## Handoff

§ 7.1
W11 builds the TLS/TOFU layer on this scaffold.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
