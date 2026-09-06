# Step: 01-step-src-workspace

## Ownership

- Goal: `06-shared-proto`
- Work unit: `W39`
- Type: `config`

## Change target

- File: `src/Cargo.toml`
- Primary symbol or file scope: `cargo workspace members`
- Subscope: `N/A`

## Objective

§ 4.1
Create src/Cargo.toml as a cargo workspace (resolver 2) with members chat-server-rs, chat-client-rs and chat-proto, so server and client share one protocol lib and dependencies are pinned once. Keep each crates own Cargo.toml so the manifest-path build still works.

## Instructions

§ 5.1
Create src/Cargo.toml with [workspace] resolver 2 and members chat-proto and chat-server-rs (chat-client-rs is added in goal 02 once the crate exists). Move the release profile (opt-level z, lto, strip, panic abort) to the workspace root so cargo does not warn about a non-root package profile. Keep src/chat-server-rs/Cargo.toml intact so cargo build --manifest-path still works.

## Acceptance criteria

§ 6.1
cargo build --workspace succeeds from src/ with no profile warning; chat-server-rs depends on chat-proto; cargo build --release --manifest-path src/chat-server-rs/Cargo.toml still succeeds.

## Handoff

§ 7.1
Goal 02 may add chat-client-rs to the workspace members list. Goal 01 builds chat-server-rs on this workspace.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
