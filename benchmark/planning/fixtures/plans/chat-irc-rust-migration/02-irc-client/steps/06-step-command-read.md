# Step: 06-step-command-read

## Ownership

- Goal: `02-irc-client`
- Work unit: `W16`
- Type: `source`

## Change target

- File: `src/chat-client-rs/src/main.rs`
- Primary symbol or file scope: `read-delta command CLI`
- Subscope: `N/A`

## Objective

§ 4.1
Implement the `read <server> #chan --since <id>` CLI action: connect via W11 TLS, emit the FETCH history request (W12 helper), and print rows with id > since.

## Instructions

§ 5.1
read: connect (TOFU), wait for welcome, send FETCH #chan <since>, print each MSG row, and stop on the FETCH_END marker.

## Acceptance criteria

§ 6.1
read -since 0 prints rows with id > 0 and stops on :server 000 end-of-history #chan.

## Handoff

§ 7.1
W17 tails the channel.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
