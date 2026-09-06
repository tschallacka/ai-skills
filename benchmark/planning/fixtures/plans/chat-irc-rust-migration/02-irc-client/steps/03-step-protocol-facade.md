# Step: 03-step-protocol-facade

## Ownership

- Goal: `02-irc-client`
- Work unit: `W12`
- Type: `source`

## Change target

- File: `src/chat-client-rs/src/main.rs`
- Primary symbol or file scope: `protocol response facade (parse prefix/numeric/replies)`
- Subscope: `N/A`

## Objective

§ 4.1
Use the shared src/chat-proto crate (same types as the server) to parse server responses: `:prefix CMD params :trailing`, numerics 001-005/353/366/433/375/372/376, and the FETCH reply rows plus the terminating marker `:server 000 end-of-history #chan` (the client stops reading on it). Expose helpers for register/join/privmsg/send/read-delta so commands in W14/W16/W17 are thin. Do NOT duplicate protocol types in the client.

## Instructions

§ 5.1
Use chat-proto::Message to parse server numeric/prefix replies (001-005,353,366,433,375,372,376) and the FETCH rows+terminating marker; expose helpers so commands stay thin.

## Acceptance criteria

§ 6.1
The client parses numeric replies and stops the delta read on the FETCH_END marker; no duplicated protocol types.

## Handoff

§ 7.1
W13/W14/W16/W17 use these helpers.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
