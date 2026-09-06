# Goal: IRG-compliant TLS rust chat server with UDP announce

## Current state and prior-goal handoffs

§ 2.1
`src/chat-server-rs` builds clean (flake.nix 1.97 toolchain, clippy -D warnings after 3 trivial fixes). It speaks a custom protocol (NICK/JOIN/LEAVE/PRIVMSG/FETCH/PING/QUIT, bare OK/ERR, MSG log lines), binds 127.0.0.1, no TLS, no UDP announce. No prior goal. Baseline unit test: chat/tests/test-chat.sh exercises the compiled rung with a raw-socket probe.

## Outcome and definition of done

§ 3.1
The server is RFC-1459-grammar compliant over TLS. A THIRD-PARTY standard IRC client that supports TLS (irssi/WeeChat/HexChat) can connect, register (NICK+USER), receive 001/002/003/004/005 + MOTD (375/372/376), join a channel, receive NAMES 353/366, send/receive PRIVMSG in `:nick!user@host PRIVMSG #chan :text` form, and PING/PONG. The additive history command FETCH #chan <since> returns messages with id > since. The server also broadcasts a UDP announce beacon on port 7780 (JSON proto ai-chat/1, plus irssi-compatible fields not required). TOFU: server generates+persists a self-signed cert on first run. Definition of done: a real TLS IRC client (or exact byte-sequence fixture) completes register/join/message; rust client FETCH returns correct delta; announce beacon observed.

## Why this goal is needed

§ 4.1
The current custom wire protocol cannot be reached by off-the-shelf IRC tooling, closing the chat to agents that only speak standard IRC over TLS. Making the server RFC-grammar + add an additive history extension keeps the agent delta-read workflow while opening interoperability.

## Scope

§ 5.1
In: RFC message grammar, registration numerics, channel JOIN/PART/NAMES/PRIVMSG/NOTICE/PING-PONG, TLS listener + self-signed cert, UDP announce task, additive FETCH. Out: MODE/op management, IRCv3 caps, SASL, DCC, TOPIC persistence, multi-server federation, keeping any interpreter/bash server tier.

## Affected files, systems, data, and interfaces

§ 6.1
src/chat-server-rs/Cargo.toml (add rustls, webpki-roots, ring), src/chat-server-rs/src/main.rs (replace custom handler with RFC lifecycle + TLS + announce thread). Possibly a new shared protocol module (src/chat-lib) or inline; decided in design. chat/tests/test-chat.sh (rewrite server portion).

## Dependencies and handoffs

§ 7.1
Depends on: none (first goal). Handoff to: 02-irc-client (client relies on the server TLS handshake + FETCH + announce); 03-legacy-removal (server replaces interpreter tiers). Shared protocol shape (module location) is decided here and used by the client — coordinate so 02 uses the same types.

## Implementation approach, risks, and edge cases

§ 8.1
Approach: single Cargo.toml per crate (mirror tony-the-pony layout); add rustls with ring backend (avoid aws-lc-rs C toolchain). Cert: generate self-signed at first run, persist to AI_CHAT_HOME, reuse on restart. Optional client TLS verification: server sends its cert; clients may verify or pin. Threading: one thread per connection (as now) with a TLS accept loop bounded; announce runs on its own thread. Risk: exact numeric/prefix form must match stock clients — mitigated by an exact byte fixture. Risk: rustls spawn requires an Accept stream wrapper — handle with a blocking accept then handshake per connection. Risk: cert regeneration must be stable so TOFU pinning persists (do NOT regenerate if a cert file exists).

## Owned work units

§ 9.1
`W02` — Wire the server to the shared src/chat-proto lib: parse incoming `:prefix CMD arg :trailing` lines through chat-proto and serialize outgoing numeric/prefix replies through it, so the server and client share one protocol definition. Remove any locally-duplicated protocol types from main.rs (single source of truth = chat-proto).

§ 9.2
`W01` — Add TLS/runtime deps to src/chat-server-rs/Cargo.toml from the src workspace: rustls 0.23 with default-features=false and features=["ring","std","tls12"] (ring backend, NOT the aws-lc-rs default), rustls-pki-types, and ring. Do NOT add rcgen/yasna (not in the offline cache) and do NOT add webpki-roots (TOFU pins the server cert directly). Keep the release profile (opt-level z, lto, strip, panic abort). Wire the crate into the src workspace and depend on the shared chat-proto crate.

§ 9.3
`W03` — Implement RFC registration: on NICK+USER set the nick/user, reply 001/002/003/004 + 005 ISUPPORT; 433 on nick-in-use; deliver MOTD via 375/372/376; reject a duplicate NICK with ERR. ALSO add an explicit CAP arm: on CAP LS 302 answer 410 (or otherwise not reject it), and on CAP END proceed — so a real client that negotiates caps before NICK/USER registers cleanly and still gets a valid 005. Standard prefix `:server 001 nick :Welcome...`.

§ 9.4
`W04` — Implement JOIN/PART/NAMES (reply 353 names + 366 end-of-names, using `:nick!user@host` prefix form for user-visible messages), and emit PRIVMSG/NOTICE to channel members in `:nick!user@host PRIVMSG #chan :text` form; persist to the channel log; honour PING/PONG. Follows RFC 1459 grammar; the log line may retain the MSG <chan> <id> <ts> <nick> :text format internally.

§ 9.5
`W05` — Add the additive non-standard command FETCH #chan <since>: reply each stored message with id > since (strictly greater — the delta contract), followed by ONE fixed terminating marker line, byte-exactly: `:server 000 end-of-history #chan` (the rust client recognizes this exact line and stops; it is never sent by a standard client).

§ 9.6
`W06` — Wrap the accept loop in rustls: at first run mint a self-signed certificate IN-CRATE via the rcgen crate (ring-backed added to Cargo.toml; no external binary, no openssl) and write server.crt/server.key under AI_CHAT_HOME; do NOT regenerate if they exist (TOFU stability). Per-connection: do a TLS handshake before the protocol loop; keep the port file contract (bare digits).

§ 9.7
`W07` — On a separate thread, broadcast a JSON beacon {"proto":"ai-chat/1","name":...,"port":...,"started":...} to UDP port 7780 every interval (configurable, default 2s), so clients can discover the server; bind SO_BROADCAST (or loopback for tests).

§ 9.8
`W08` — Verify a THIRD-PARTY standard TLS IRC client (or a faithful byte fixture + `openssl s_client -verify_quiet -connect 127.0.0.1:<port> -servername localhost`) can register, join and message the server. Assert the exact byte sequence a real client expects: NICK+USER gives 001-005 (including a valid 005 ISUPPORT), 375/372/376 MOTD, 353 names + 366 end-of-names, and `:nick!user@host PRIVMSG #chan :text` on the wire, and that a client that sends CAP LS 302 / CAP END before NICK/USER is still registered (the server must answer CAP with 410 or NOT reject it, and issue a valid 005). NOTE: the correct openSSL option is -verify_quiet (there is no -no-cert-check). This is the acceptance proof that a stock client interoperates.

§ 9.9
`W09` — Verify FETCH #chan <since> returns messages with id > since and the UDP announce beacon is receivable (loopback). Run after 01-08 so the server is proven client-correct.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | verify the server with a real TLS IRC client / byte fixture and assert the full numeric sequence, prefix form, FETCH delta, and announce beacon. |
## Goal-size exception
