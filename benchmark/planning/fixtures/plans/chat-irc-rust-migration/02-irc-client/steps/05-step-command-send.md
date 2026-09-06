# Step: 05-step-command-send

## Ownership

- Goal: `02-irc-client`
- Work unit: `W14`
- Type: `source`

## Change target

- File: `src/chat-client-rs/src/main.rs`
- Primary symbol or file scope: `send command CLI`
- Subscope: `N/A`

## Objective

§ 4.1
Implement the `send <server> #chan :text` CLI action: connect via W11 TLS, send PRIVMSG with the message text, and report the stored line; use W12 protocol helpers.

## Instructions

§ 5.1
send: connect (TOFU), wait for welcome, JOIN the channel, PRIVMSG the text, and report the echoed stored line.

## Acceptance criteria

§ 6.1
send prints the :nick!user@host PRIVMSG #chan :text echo and the message is persisted.

## Handoff

§ 7.1
W16 reads the message back.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
