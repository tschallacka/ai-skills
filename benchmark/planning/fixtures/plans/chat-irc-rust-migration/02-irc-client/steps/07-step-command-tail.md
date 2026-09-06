# Step: 07-step-command-tail

## Ownership

- Goal: `02-irc-client`
- Work unit: `W17`
- Type: `source`

## Change target

- File: `src/chat-client-rs/src/main.rs`
- Primary symbol or file scope: `tail command CLI`
- Subscope: `N/A`

## Objective

§ 4.1
Implement the `tail <server> #chan [since-id]` CLI action: connect via W11 TLS, JOIN the channel (subscribing), read pushed messages (or poll), and stream them until interrupted; use W12 protocol helpers.

## Instructions

§ 5.1
tail: connect (TOFU), wait for welcome, JOIN the channel, then stream pushed PRIVMSG lines until the connection closes.

## Acceptance criteria

§ 6.1
A joined tail streams PRIVMSG lines as other clients send them.

## Handoff

§ 7.1
W15 verifies the client end-to-end.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
