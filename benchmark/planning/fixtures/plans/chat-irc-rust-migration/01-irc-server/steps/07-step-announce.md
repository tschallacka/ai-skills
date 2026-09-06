# Step: 07-step-announce

## Ownership

- Goal: `01-irc-server`
- Work unit: `W07`
- Type: `source`

## Change target

- File: `src/chat-server-rs/src/main.rs`
- Primary symbol or file scope: `UDP announce beacon task`
- Subscope: `N/A`

## Objective

§ 4.1
On a separate thread, broadcast a JSON beacon {"proto":"ai-chat/1","name":...,"port":...,"started":...} to UDP port 7780 every interval (configurable, default 2s), so clients can discover the server; bind SO_BROADCAST (or loopback for tests).

## Instructions

§ 5.1
On a separate thread, broadcast a JSON beacon {"proto":"ai-chat/1","name":...,"port":...,"started":...} on UDP port 7780 (configurable CHAT_BEACON_PORT, CHAT_BCAST, interval) so clients can discover the server.

## Acceptance criteria

§ 6.1
The announce beacon is receivable on the configured UDP port as {"proto":"ai-chat/1",...}.

## Handoff

§ 7.1
The client (goal 02 W13) discovers the server via this beacon.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
