# Step: 06-step-verify-client

## Ownership

- Goal: `02-irc-client`
- Work unit: `W15`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `client end-to-end against goal-01 server`
- Subscope: `N/A`

## Objective

§ 4.1
Verify the client discovers the goal-01 server via UDP beacon (loopback), connects over TLS with TOFU pinning (succeeds first connect, rejects a changed fingerprint unless --insecure), sends a PRIVMSG, reads a delta since an id, and tails a channel. Asserted in chat/tests/test-chat.sh.

## Instructions

§ 5.1
Run the client discover/send/read-delta/tail against the goal-01 server over TLS with TOFU; assert discovery, send echo, delta since-an-id, and a streamed tail.

## Acceptance criteria

§ 6.1
discover lists the server; send echoes; read returns the delta; tail streams; the cert is pinned on first connect.

## Handoff

§ 7.1
Goal 03 removes the bash client helpers in favour of this client.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
