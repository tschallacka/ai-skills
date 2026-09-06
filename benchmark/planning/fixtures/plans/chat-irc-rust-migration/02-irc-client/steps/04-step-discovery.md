# Step: 04-step-discovery

## Ownership

- Goal: `02-irc-client`
- Work unit: `W13`
- Type: `source`

## Change target

- File: `src/chat-client-rs/src/main.rs`
- Primary symbol or file scope: `UDP discovery (listen for announce beacon)`
- Subscope: `N/A`

## Objective

§ 4.1
Bind a UDP socket to the beacon port (7780), read JSON beacons for a window, dedupe by name+port, and list servers (human or --json); allow --bcast/--beacon-port for loopback tests.

## Instructions

§ 5.1
Bind a UDP socket to the beacon port (7780, or --beacon-port), read JSON {"proto":"ai-chat/1","name":...,"port":...} beacons for --wait seconds, dedupe by name+port, and print the list (human or --json).

## Acceptance criteria

§ 6.1
discover against the goal-01 announce beacon lists the server by name+port.

## Handoff

§ 7.1
W14 sends to the discovered server.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
