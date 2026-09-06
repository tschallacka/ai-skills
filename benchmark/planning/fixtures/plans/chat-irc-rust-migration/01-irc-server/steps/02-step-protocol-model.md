# Step: 02-step-protocol-model

## Ownership

- Goal: `01-irc-server`
- Work unit: `W02`
- Type: `source`

## Change target

- File: `src/chat-server-rs/src/main.rs`
- Primary symbol or file scope: server adapter to chat-proto
- Subscope: `N/A`

## Objective

§ 4.1
Wire the server to the shared src/chat-proto lib: parse incoming `:prefix CMD arg :trailing` lines through chat-proto and serialize outgoing numeric/prefix replies through it, so the server and client share one protocol definition. Remove any locally-duplicated protocol types from main.rs (single source of truth = chat-proto).

## Instructions

§ 5.1
Use chat_proto::Message to parse incoming lines and serialize numeric/prefix replies; remove any locally-duplicated protocol types from main.rs.

## Acceptance criteria

§ 6.1
The server parses :prefix CMD params :trailing via chat-proto and emits numerics with the standard prefix form; no duplicated protocol struct in main.rs.

## Handoff

§ 7.1
W03/W04 build the lifecycle on this shared model.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
