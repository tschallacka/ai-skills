# Goal: IRC-compliant TLS rust chat client with UDP discovery

## Current state and prior-goal handoffs

§ 2.1
No rust chat client exists. `src/tony-the-pony` is an unrelated tool-call gate. Handoff from 01-irc-server: the server speaks RFC grammar over TLS, exposes FETCH #chan <since>, and broadcasts a UDP announce beacon on 7780; its self-signed cert is persisted at AI_CHAT_HOME. The client must match that protocol exactly.

## Outcome and definition of done

§ 3.1
A rust chat client (`src/chat-client-rs`) connects to the IRC server over TLS, discovers it via the UDP beacon, pins the server cert on first connect (TOFU), registers (NICK/USER), joins a channel, sends PRIVMSG, reads a delta since an id (FETCH), and tails a channel. Definition of done: client, built by cargo, succeeds against the 01 server for discovery + send + delta-read + tail; clippy -D warnings clean; cert pinned so a second connect does not re-prompt.

## Why this goal is needed

§ 4.1
Replaces the bash client helpers with a single compiled, portable binary; without it the legacy removal (goal 03) leaves the skill with no way to talk to the server.

## Scope

§ 5.1
In: TLS connection + TOFU cert pinning; UDP discovery (listen for announce beacon, list servers); sending a message; reading a delta since an id; tailing. Out: a full interactive ncurses client; multi-server join; server-side functionality; IRCv3/SASL.

## Affected files, systems, data, and interfaces

§ 6.1
New crate: src/chat-client-rs/{Cargo.toml,rust-toolchain.toml,Cargo.lock,src/main.rs}. Deps: rustls, rustls-pki-types, webpki-roots, ring. chat/tests/test-chat.sh (client portion). scripts/chat-server.sh is removed in goal 03, so the client communicates directly (not via the old bash launcher).

## Dependencies and handoffs

§ 7.1
Depends on: 01-irc-server (protocol, TLS, announce, FETCH). Handoff to: 03-legacy-removal (client replaces bash send/read/tail/register/watch).

## Implementation approach, risks, and edge cases

§ 8.1
Approach: mirror chat-server-rs conventions (single Cargo.toml, rust-toolchain 1.97, MODE markers, clippy). TOFU: store the pinned cert fingerprint under AI_CHAT_HOME; on connect, verify the server cert against the stored fingerprint; on first connect, persist it (with a flag to bypass for testing). Discovery: bind UDP, read beacons for a window, dedupe by name+port, print list (human) or JSON. Send/delta/tail: send PRIVMSG; FETCH then format; tail = subscribe (JOIN) + read pushed lines, or poll. Risk: rustls client-server cert verification — use a NoVerify or pinned approach; a public-suffix trust error appears if using system roots, so pin. Risk: cert mismatch after server cert regenerates — record regen as breaking change.

## Owned work units

§ 9.1
`W10` — Create the client crate scaffold: src/chat-client-rs/{Cargo.toml,rust-toolchain.toml(src workspace 1.97, rustfmt+clippy),src/main.rs}, MODE: DEV / PACKAGE: PROD headers, release profile opt-level z/lto/strip/panic abort. Dependencies: rustls 0.23 (default-features=false, features=["ring","std","tls12"]), rustls-pki-types, ring, and the shared src/chat-proto crate. NOT webpki-roots (TOFU pins the cert).

§ 9.2
`W11` — Connect to host:port over TLS (rustls); on first connection store the server cert fingerprint under AI_CHAT_HOME (e.g. <server>.cert.fp) and succeed; on later connections require the fingerprint to match, with a --insecure/no-verify flag for testing. Fail closed otherwise.

§ 9.3
`W12` — Use the shared src/chat-proto crate (same types as the server) to parse server responses: `:prefix CMD params :trailing`, numerics 001-005/353/366/433/375/372/376, and the FETCH reply rows plus the terminating marker `:server 000 end-of-history #chan` (the client stops reading on it). Expose helpers for register/join/privmsg/send/read-delta so commands in W14/W16/W17 are thin. Do NOT duplicate protocol types in the client.

§ 9.4
`W13` — Bind a UDP socket to the beacon port (7780), read JSON beacons for a window, dedupe by name+port, and list servers (human or --json); allow --bcast/--beacon-port for loopback tests.

§ 9.5
`W15` — Verify the client discovers the goal-01 server via UDP beacon (loopback), connects over TLS with TOFU pinning (succeeds first connect, rejects a changed fingerprint unless --insecure), sends a PRIVMSG, reads a delta since an id, and tails a channel. Asserted in chat/tests/test-chat.sh.

§ 9.6
`W14` — Implement the `send <server> #chan :text` CLI action: connect via W11 TLS, send PRIVMSG with the message text, and report the stored line; use W12 protocol helpers.

§ 9.7
`W17` — Implement the `tail <server> #chan [since-id]` CLI action: connect via W11 TLS, JOIN the channel (subscribing), read pushed messages (or poll), and stream them until interrupted; use W12 protocol helpers.

§ 9.8
`W16` — Implement the `read <server> #chan --since <id>` CLI action: connect via W11 TLS, emit the FETCH history request (W12 helper), and print rows with id > since.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | verify: client discovers the server via UDP beacon, connects over TLS with TOFU pinning, sends, reads a delta, and tails — asserted in chat/tests/test-chat.sh. |
## Goal-size exception
