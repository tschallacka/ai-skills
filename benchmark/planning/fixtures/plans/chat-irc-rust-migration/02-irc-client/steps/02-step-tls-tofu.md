# Step: 02-step-tls-tofu

## Ownership

- Goal: `02-irc-client`
- Work unit: `W11`
- Type: `source`

## Change target

- File: `src/chat-client-rs/src/main.rs`
- Primary symbol or file scope: `TLS connect + TOFU cert pinning`
- Subscope: `N/A`

## Objective

§ 4.1
Connect to host:port over TLS (rustls); on first connection store the server cert fingerprint under AI_CHAT_HOME (e.g. <server>.cert.fp) and succeed; on later connections require the fingerprint to match, with a --insecure/no-verify flag for testing. Fail closed otherwise.

## Instructions

§ 5.1
Connect over TLS; after the handshake read the peer cert fingerprint, store it under AI_CHAT_HOME/<server>.cert.fp on first connect, and require an exact match on later connects; --insecure bypasses the pin. Fail closed (exit 70) on a mismatch.

## Acceptance criteria

§ 6.1
First connect writes the pin file; a second connect to the same server succeeds; a mismatched pin is rejected with an error.

## Handoff

§ 7.1
W12 uses the established TLS stream for the protocol.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
