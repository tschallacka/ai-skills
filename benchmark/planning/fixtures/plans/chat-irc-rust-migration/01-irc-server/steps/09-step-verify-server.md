# Step: 09-step-verify-server

## Ownership

- Goal: `01-irc-server`
- Work unit: `W09`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `server delta + announce verification`
- Subscope: `N/A`

## Objective

§ 4.1
Verify FETCH #chan <since> returns messages with id > since and the UDP announce beacon is receivable (loopback). Run after 01-08 so the server is proven client-correct.

## Instructions

§ 5.1
Verify FETCH #chan <since> returns rows with id > since followed by the FETCH_END marker, and that the announce beacon is receivable on loopback.

## Acceptance criteria

§ 6.1
FETCH returns id > since and the terminating marker; the announce beacon is received.

## Handoff

§ 7.1
The server delta+announce contract is proven for the client.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
