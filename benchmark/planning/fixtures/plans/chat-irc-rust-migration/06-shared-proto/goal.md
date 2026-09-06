# Goal: Shared src workspace and chat-proto protocol crate

## Current state and prior-goal handoffs

§ 2.1
chat-server-rs and tony-the-pony are independent crates at src/<name>/ with their own Cargo.toml and no workspace. No shared protocol type exists: the server parses/serializes in main.rs and the planned client would duplicate it (the source of the drift risk flagged by the adversary).

## Outcome and definition of done

§ 3.1
src/Cargo.toml is a cargo workspace with members chat-server-rs, chat-client-rs and chat-proto; chat-proto is a lib crate with `parse` and `serialize` for the RFC-grammar message (prefix form) and the numeric tags + FETCH reply rows. Server and client both depend on chat-proto and do NOT duplicate protocol types. Definition of done: cargo build --workspace succeeds and a cargo test in chat-proto round-trips a message.

## Why this goal is needed

§ 4.1
A single source of truth is the only way to guarantee the client and server agree on framing, numerics and the additive FETCH contract; the adversary explicitly flagged this as a blocker.

## Scope

§ 5.1
In: src/Cargo.toml workspace; src/chat-proto crate (parse+serialize+tags+FETCH rows); server/client depend on it. Out: any behavior logic (channels, TLS, clients).

## Affected files, systems, data, and interfaces

§ 6.1
src/Cargo.toml (new), src/chat-proto/Cargo.toml, src/chat-proto/rust-toolchain.toml, src/chat-proto/src/message.rs; src/chat-server-rs/Cargo.toml and src/chat-client-rs/Cargo.toml gain path deps on chat-proto.

## Dependencies and handoffs

§ 7.1
Depends on: none (first). Handoff to 01-irc-server and 02-irc-client: both consume chat-proto.

## Implementation approach, risks, and edge cases

§ 8.1
Approach: create src/Cargo.toml workspace (resolver 2) listing the members; add chat-proto lib; add path deps to server+client. Risk: moving chat-server-rs into a workspace changes its build path; keep each crates own Cargo.toml so `cargo build --manifest-path src/chat-server-rs/Cargo.toml` still works (the installer and tests use it).

## Owned work units

§ 9.1
`W39` — Create src/Cargo.toml as a cargo workspace (resolver 2) with members chat-server-rs, chat-client-rs and chat-proto, so server and client share one protocol lib and dependencies are pinned once. Keep each crates own Cargo.toml so the manifest-path build still works.

§ 9.2
`W40` — Create src/chat-proto as a lib crate (MODE DEV/PACKAGE PROD, pinned toolchain). It models an IRC message: parse `:prefix CMD params :trailing` and serialize it, and centralize the numeric tags (001-005,353,366,433,375,372,376) and the FETCH reply rows. Both the server and client depend on this crate.

§ 9.3
`W44` — Verify src/ builds as a workspace and chat-proto round-trips a message: cargo build --workspace --release succeeds and cargo test in src/chat-proto parses and re-serializes a `:prefix CMD params :trailing` line (and the numeric tags) byte-equivalently; clippy -D warnings clean across the workspace.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | cargo build --workspace and a chat-proto unit test that round-trips a message; clippy -D warnings across the workspace. |

## Goal-size exception
