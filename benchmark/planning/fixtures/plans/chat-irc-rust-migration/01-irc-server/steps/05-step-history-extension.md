# Step: 05-step-history-extension

## Ownership

- Goal: `01-irc-server`
- Work unit: `W05`
- Type: `source`

## Change target

- File: `src/chat-server-rs/src/main.rs`
- Primary symbol or file scope: `history extension command (FETCH #chan <since>)`
- Subscope: `N/A`

## Objective

§ 4.1
Add the additive non-standard command FETCH #chan <since>: reply each stored message with id > since (strictly greater — the delta contract), followed by ONE fixed terminating marker line, byte-exactly: `:server 000 end-of-history #chan` (the rust client recognizes this exact line and stops; it is never sent by a standard client).

## Instructions

§ 5.1
Add FETCH #chan <since>: reply each stored message with id > since (delta contract), then the single terminating marker :server 000 end-of-history #chan (byte-exact, shared via chat-proto FETCH_END). A standard client never sends FETCH.

## Acceptance criteria

§ 6.1
FETCH #chan 0 returns rows with id > 0 then :server 000 end-of-history #chan; the marker line is exactly the FETCH_END constant.

## Handoff

§ 7.1
The client (goal 02) reads the marker to stop its delta read.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
