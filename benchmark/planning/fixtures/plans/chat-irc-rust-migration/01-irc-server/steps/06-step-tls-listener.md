# Step: 06-step-tls-listener

## Ownership

- Goal: `01-irc-server`
- Work unit: `W06`
- Type: `source`

## Change target

- File: `src/chat-server-rs/src/main.rs`
- Primary symbol or file scope: `TLS listener + self-signed cert`
- Subscope: `N/A`

## Objective

§ 4.1
Wrap the accept loop in rustls: at first run mint a self-signed certificate IN-CRATE via the rcgen crate (ring-backed added to Cargo.toml; no external binary, no openssl) and write server.crt/server.key under AI_CHAT_HOME; do NOT regenerate if they exist (TOFU stability). Per-connection: do a TLS handshake before the protocol loop; keep the port file contract (bare digits).

## Instructions

§ 5.1
Wrap the accept loop in rustls: at first run mint a self-signed cert via the openssl CLI (encrypted into AI_CHAT_HOME/server.crt+key, NO regeneration if present), load into a rustls ServerConfig, and per-connection do a TLS handshake before the protocol loop; keep the port file as bare digits; declare openssl a runtime requirement.

## Acceptance criteria

§ 6.1
The server listens TLS-only; the cert is created once and reused across restarts; a non-TLS connect fails the handshake; server.port holds bare digits.

## Handoff

§ 7.1
W07 announce runs alongside the TLS listener; the client (goal 02) pins this cert via TOFU.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
