# Work-unit inventory: chat-irc-rust-migration

## Definition-of-done coverage

| Required outcome or proof | Work unit IDs | Notes |
|---|---|---|

| 01-irc-server: server is RFC-grammar IRC over TLS reachable by a standard TLS IRC client | W01,W02,W03,W04,W06,W08 | tls deps + protocol model + registration + channels + tls listener covered; W08 asserts the standard-client proof |

| 01-irc-server: additive history extension returns delta | W05,W09 | FETCH implemented (W05); verified delta + announce (W09) |

| 01-irc-server: UDP announce beacon broadcast | W07,W09 | announce task (W07) + receive verify (W09) |

| 02-irc-client: uses TLS + TOFU pinning | W10,W11,W15 | scaffold + tls-tofu + end-to-end verify |

| 02-irc-client: UDP discovery finds server | W13,W15 | discovery + e2e verify |

| 02-irc-client: send / read-delta / tail work | W12,W14,W16,W17,W15 | protocol facade + send + read-delta + tail + e2e verify (W15) |

| 03-legacy-removal: bash client helpers + launcher removed | W20,W22,W23,W24,W25,W26,W27,W28 | the eight bash scripts deleted |

| 04-remove-interpreter-tiers: interpreter server tiers removed | W29,W30,W31,W32 | the four interpreter files deleted |

| 05-update-consumers: manifest/installer/docs/tests ship rust-only | W21,W33,W34,W35,W36,W37,W38 | consumers updated + test rewritten; W38 proves the rust path |

| 01-irc-server: src workspace + shared proto so server/client can not drift |W02,W12| workspace (W39), proto crate (W40), server adapter (W02), client facade (W12) |


| 06-shared-proto: workspace + shared proto so server and client can not drift | W39,W40,W44 | workspace (W39), proto crate (W40), workspace/proto verify (W44) |

| 01-irc-server: server adopts the shared chat-proto wire format | W02 | server adapter to chat-proto |

| 04-remove-interpreter-tiers: built chat binaries stay gitignored | W46 | gitignore rule keeps the built binaries untracked |

| 05-update-consumers: release ships the prebuilt rust binaries (never built at install) | W21,W43,W45,W46,W38 | installer source (W21), portability allowlist (W43), release-build synthesis (W45), gitignore (W46), test rewrite (W38) |

## Work units

| ID | Type | File | Primary symbol or file scope | Subscope | Intended change | Depends on | Goal | Step |
|---|---|---|---|---|---|---|---|---|

| W02 | source | `src/chat-server-rs/src/main.rs` | server adapter to chat-proto | `N/A` | Wire the server to the shared src/chat-proto lib: parse incoming `:prefix CMD arg :trailing` lines through chat-proto and serialize outgoing numeric/prefix replies through it, so the server and client share one protocol definition. Remove any locally-duplicated protocol types from main.rs (single source of truth = chat-proto). |—| 01-irc-server | 02-step-protocol-model |

| W01 | config | `src/chat-server-rs/Cargo.toml` | `[dependencies]` | `N/A` | Add TLS/runtime deps to src/chat-server-rs/Cargo.toml from the src workspace: rustls 0.23 with default-features=false and features=["ring","std","tls12"] (ring backend, NOT the aws-lc-rs default), rustls-pki-types, and ring. Do NOT add rcgen/yasna (not in the offline cache) and do NOT add webpki-roots (TOFU pins the server cert directly). Keep the release profile (opt-level z, lto, strip, panic abort). Wire the crate into the src workspace and depend on the shared chat-proto crate. | -- | 01-irc-server | 01-step-add-tls-deps |

| W03 | source | `src/chat-server-rs/src/main.rs` | `registration lifecycle (NICK/USER backend)` | `N/A` | Implement RFC registration: on NICK+USER set the nick/user, reply 001/002/003/004 + 005 ISUPPORT; 433 on nick-in-use; deliver MOTD via 375/372/376; reject a duplicate NICK with ERR. ALSO add an explicit CAP arm: on CAP LS 302 answer 410 (or otherwise not reject it), and on CAP END proceed — so a real client that negotiates caps before NICK/USER registers cleanly and still gets a valid 005. Standard prefix `:server 001 nick :Welcome...`. | W02 | 01-irc-server | 03-step-registration |

| W04 | source | `src/chat-server-rs/src/main.rs` | `channel commands (JOIN/PART/NAMES/PRIVMSG/NOTICE/PING/PONG)` | `N/A` | Implement JOIN/PART/NAMES (reply 353 names + 366 end-of-names, using `:nick!user@host` prefix form for user-visible messages), and emit PRIVMSG/NOTICE to channel members in `:nick!user@host PRIVMSG #chan :text` form; persist to the channel log; honour PING/PONG. Follows RFC 1459 grammar; the log line may retain the MSG <chan> <id> <ts> <nick> :text format internally. | W03 | 01-irc-server | 04-step-channels |

| W05 | source | `src/chat-server-rs/src/main.rs` | `history extension command (FETCH #chan <since>)` | `N/A` | Add the additive non-standard command FETCH #chan <since>: reply each stored message with id > since (strictly greater — the delta contract), followed by ONE fixed terminating marker line, byte-exactly: `:server 000 end-of-history #chan` (the rust client recognizes this exact line and stops; it is never sent by a standard client). | W02 | 01-irc-server | 05-step-history-extension |

| W06 | source | `src/chat-server-rs/src/main.rs` | `TLS listener + self-signed cert` | `N/A` | Wrap the accept loop in rustls: at first run mint a self-signed certificate IN-CRATE via the rcgen crate (ring-backed added to Cargo.toml; no external binary, no openssl) and write server.crt/server.key under AI_CHAT_HOME; do NOT regenerate if they exist (TOFU stability). Per-connection: do a TLS handshake before the protocol loop; keep the port file contract (bare digits). | W01 | 01-irc-server | 06-step-tls-listener |

| W07 | source | `src/chat-server-rs/src/main.rs` | `UDP announce beacon task` | `N/A` | On a separate thread, broadcast a JSON beacon {"proto":"ai-chat/1","name":...,"port":...,"started":...} to UDP port 7780 every interval (configurable, default 2s), so clients can discover the server; bind SO_BROADCAST (or loopback for tests). | W06 | 01-irc-server | 07-step-announce |

| W08 | verification | `N/A` | `standard TLS IRC client connect (byte-sequence fixture)` | `N/A` | Verify a THIRD-PARTY standard TLS IRC client (or a faithful byte fixture + `openssl s_client -verify_quiet -connect 127.0.0.1:<port> -servername localhost`) can register, join and message the server. Assert the exact byte sequence a real client expects: NICK+USER gives 001-005 (including a valid 005 ISUPPORT), 375/372/376 MOTD, 353 names + 366 end-of-names, and `:nick!user@host PRIVMSG #chan :text` on the wire, and that a client that sends CAP LS 302 / CAP END before NICK/USER is still registered (the server must answer CAP with 410 or NOT reject it, and issue a valid 005). NOTE: the correct openSSL option is -verify_quiet (there is no -no-cert-check). This is the acceptance proof that a stock client interoperates. | W04 | 01-irc-server | 08-step-verify-standard-client |

| W09 | verification | `N/A` | `server delta + announce verification` | `N/A` | Verify FETCH #chan <since> returns messages with id > since and the UDP announce beacon is receivable (loopback). Run after 01-08 so the server is proven client-correct. | W05,W07 | 01-irc-server | 09-step-verify-server |

| W10 | config | `src/chat-client-rs/Cargo.toml` | `[package]+[dependencies]` | `N/A` | Create the client crate scaffold: src/chat-client-rs/{Cargo.toml,rust-toolchain.toml(src workspace 1.97, rustfmt+clippy),src/main.rs}, MODE: DEV / PACKAGE: PROD headers, release profile opt-level z/lto/strip/panic abort. Dependencies: rustls 0.23 (default-features=false, features=["ring","std","tls12"]), rustls-pki-types, ring, and the shared src/chat-proto crate. NOT webpki-roots (TOFU pins the cert). | -- | 02-irc-client | 01-step-crate-scaffold |

| W11 | source | `src/chat-client-rs/src/main.rs` | `TLS connect + TOFU cert pinning` | `N/A` | Connect to host:port over TLS (rustls); on first connection store the server cert fingerprint under AI_CHAT_HOME (e.g. <server>.cert.fp) and succeed; on later connections require the fingerprint to match, with a --insecure/no-verify flag for testing. Fail closed otherwise. | W10 | 02-irc-client | 02-step-tls-tofu |

| W12 | source | `src/chat-client-rs/src/main.rs` | `protocol response facade (parse prefix/numeric/replies)` | `N/A` | Use the shared src/chat-proto crate (same types as the server) to parse server responses: `:prefix CMD params :trailing`, numerics 001-005/353/366/433/375/372/376, and the FETCH reply rows plus the terminating marker `:server 000 end-of-history #chan` (the client stops reading on it). Expose helpers for register/join/privmsg/send/read-delta so commands in W14/W16/W17 are thin. Do NOT duplicate protocol types in the client. |W11| 02-irc-client | 03-step-protocol-facade |

| W13 | source | `src/chat-client-rs/src/main.rs` | `UDP discovery (listen for announce beacon)` | `N/A` | Bind a UDP socket to the beacon port (7780), read JSON beacons for a window, dedupe by name+port, and list servers (human or --json); allow --bcast/--beacon-port for loopback tests. | W11 | 02-irc-client | 04-step-discovery |


| W15 | verification | `N/A` | `client end-to-end against goal-01 server` | `N/A` | Verify the client discovers the goal-01 server via UDP beacon (loopback), connects over TLS with TOFU pinning (succeeds first connect, rejects a changed fingerprint unless --insecure), sends a PRIVMSG, reads a delta since an id, and tails a channel. Asserted in chat/tests/test-chat.sh. | W14,W16,W17 | 02-irc-client | 08-step-verify-client |


| W20 | source | `chat/scripts/chat-server.sh` | `delete the legacy launcher` | `N/A` | git rm chat/scripts/chat-server.sh (the bash server launcher). The rust server binary is started directly by the rust client/skill; no bash launcher remains. | W15 | 03-legacy-removal | 01-step-delete-server-sh |

| W22 | source | `chat/scripts/chat-send.sh` | `delete legacy send helper` | `N/A` | git rm chat/scripts/chat-send.sh. Replaced by the rust client send command. | W15 | 03-legacy-removal | 03-step-delete-send |

| W23 | source | `chat/scripts/chat-read.sh` | `delete legacy read helper` | `N/A` | git rm chat/scripts/chat-read.sh. Replaced by the rust client read-delta command. | W15 | 03-legacy-removal | 04-step-delete-read |

| W24 | source | `chat/scripts/chat-tail.sh` | `delete legacy tail helper` | `N/A` | git rm chat/scripts/chat-tail.sh. Replaced by the rust client tail command. | W15 | 03-legacy-removal | 05-step-delete-tail |

| W25 | source | `chat/scripts/chat-register.sh` | `delete legacy register helper` | `N/A` | git rm chat/scripts/chat-register.sh. Channel registration is implicit via JOIN on the rust server. | W15 | 03-legacy-removal | 06-step-delete-register |

| W26 | source | `chat/scripts/chat-watch.sh` | `delete legacy watch helper` | `N/A` | git rm chat/scripts/chat-watch.sh. Replaced by the rust client tail/poll. | W15 | 03-legacy-removal | 07-step-delete-watch |

| W27 | source | `chat/scripts/chat-announce.sh` | `delete legacy announce script` | `N/A` | git rm chat/scripts/chat-announce.sh. The rust server now broadcasts the UDP announce beacon itself. | W15 | 03-legacy-removal | 08-step-delete-announce |

| W28 | source | `chat/scripts/chat-discover.sh` | `delete legacy discover script` | `N/A` | git rm chat/scripts/chat-discover.sh. The rust client now discovers via its own UDP listener. | W15 | 03-legacy-removal | 09-step-delete-discover |











| W29 | source | `chat/runtime/server.py` | `delete python server tier` | `N/A` | git rm chat/runtime/server.py. | W07 | 04-remove-interpreter-tiers | 01-step-delete-server-py |

| W30 | source | `chat/runtime/server.js` | `delete node server tier` | `N/A` | git rm chat/runtime/server.js. | W07 | 04-remove-interpreter-tiers | 02-step-delete-server-js |

| W31 | source | `chat/runtime/server.pl` | `delete perl server tier` | `N/A` | git rm chat/runtime/server.pl. | W07 | 04-remove-interpreter-tiers | 03-step-delete-server-pl |

| W32 | source | `chat/runtime/bash-handler.sh` | `delete socat handler tier` | `N/A` | git rm chat/runtime/bash-handler.sh. | W07 | 04-remove-interpreter-tiers | 04-step-delete-bash-handler |

| W21 | config | installer/src/50-manifest.sh | skill_files chat block in installer source | `N/A` | Update the installer SOURCE — installer/src/50-manifest.sh (and installer/src/05-config.sh) — so the generated install.sh skill_files() chat block lists the prebuilt rust server and client binaries under chat/bin/ (BUILT BY THE RELEASE/CI, NEVER BY THE SHIPPED INSTALL.SH, and not committed), with NO crate-build step in skill_files() (install.sh never runs cargo: a released payload has no src/ and no cargo on the target). SKILL_NAMES/description text reflects rust-only chat. Regenerate install.sh with installer/build.sh afterwards. | W32 | 05-update-consumers | 01-step-install-sh |

| W33 | config | `chat/requires.tsv` | `requirement rows (drop interpreter/bash rows)` | `N/A` | Rewrite chat/requires.tsv: remove the bash-hard row and the python3/node/perl/socat soft server-runtimes group. Declare NO runtime tool requirement — the rust server mints its own cert in-crate (rcgen) and the client pins it via TOFU, so the shipped binaries need nothing at runtime. | W15 | 05-update-consumers | 02-step-requires-tsv |

| W34 | docs | `chat/SKILL.md` | `skill doc: rust-only server + client usage` | `N/A` | Rewrite chat/SKILL.md: describe the rust server start, rust client send/read-delta/tail/discover commands, the additive FETCH extension, TLS + TOFU, and remove all bash-helper and interpreter-fallback guidance. | W33 | 05-update-consumers | 03-step-skill-doc |

| W35 | docs | `chat/docs/README.md` | `end-user README (rust-only)` | `N/A` | Rewrite chat/docs/README.md to present the rust server + rust client (build, run, discover, send/read/tail), dropping the bash-helper and runtime-fallbacks framing. | W34 | 05-update-consumers | 04-step-readme |

| W36 | config | `README.md` | `top-level skills table row` | `N/A` | Update the README.md skills-table row for Chat to describe the rust server + rust client, removing the bash/runtime-fallbacks wording. | W34 | 05-update-consumers | 05-step-root-readme |

| W37 | config | `package.json` | `files entry` | `N/A` | Update package.json files entry for chat: include the released chat/bin/* binaries (built by the release/CI, not committed) and drop references to the deleted scripts/ and runtime/ files. Do NOT include src/ (the binaries are prebuilt). | W32 | 05-update-consumers | 06-step-package-json |

| W38 | test | `chat/tests/test-chat.sh` | `rewrite to rust server + client` | `N/A` | Rewrite chat/tests/test-chat.sh to drive only the rust server and rust client: run under `nix develop` (cargo/rustc only on PATH there), build the two binaries into chat/bin/, start the rust server, run the rust client discover/send/read-delta/tail against it, assert TLS (TOFU pinning) and FETCH delta (id > since, terminating marker), and drop all runtime/interpreter/bash expectations. | W32,W15 | 05-update-consumers | 07-step-rewrite-test |

| W14 | source | `src/chat-client-rs/src/main.rs` | `send command CLI` | `N/A` | Implement the `send <server> #chan :text` CLI action: connect via W11 TLS, send PRIVMSG with the message text, and report the stored line; use W12 protocol helpers. | W12,W13 | 02-irc-client | 05-step-command-send |


| W17 | source | `src/chat-client-rs/src/main.rs` | `tail command CLI` | `N/A` | Implement the `tail <server> #chan [since-id]` CLI action: connect via W11 TLS, JOIN the channel (subscribing), read pushed messages (or poll), and stream them until interrupted; use W12 protocol helpers. | W12,W13 | 02-irc-client | 07-step-command-tail |

| W16 | source | `src/chat-client-rs/src/main.rs` | `read-delta command CLI` | `N/A` | Implement the `read <server> #chan --since <id>` CLI action: connect via W11 TLS, emit the FETCH history request (W12 helper), and print rows with id > since. | W12,W13 | 02-irc-client | 06-step-command-read |




| W43 | config | `planning/tests/test-portability-contract.sh` | `allowlist rows for the removed chat scripts + python3 group` | `N/A` | Update tests/test-portability-contract.sh allowlist rows that reference the now-deleted chat scripts and the chat python3/server-runtime groups; remove or retarget them so the portability contract passes after the removal. Also refresh PORTABILITY.md if it enumerates chat files. | W32 | 05-update-consumers | 09-step-portability-fix |

| W39 | config | `src/Cargo.toml` | `cargo workspace members` | `N/A` | Create src/Cargo.toml as a cargo workspace (resolver 2) with members chat-server-rs, chat-client-rs and chat-proto, so server and client share one protocol lib and dependencies are pinned once. Keep each crates own Cargo.toml so the manifest-path build still works. | -- | 06-shared-proto | 01-step-src-workspace |

| W40 | source | `src/chat-proto/src/message.rs` | `Message parse/serialize + numeric tags` | `N/A` | Create src/chat-proto as a lib crate (MODE DEV/PACKAGE PROD, pinned toolchain). It models an IRC message: parse `:prefix CMD params :trailing` and serialize it, and centralize the numeric tags (001-005,353,366,433,375,372,376) and the FETCH reply rows. Both the server and client depend on this crate. | W39 | 06-shared-proto | 02-step-proto-crate |

| W44 | verification | `N/A` | `chat-proto round-trip test + workspace build` | `N/A` | Verify src/ builds as a workspace and chat-proto round-trips a message: cargo build --workspace --release succeeds and cargo test in src/chat-proto parses and re-serializes a `:prefix CMD params :trailing` line (and the numeric tags) byte-equivalently; clippy -D warnings clean across the workspace. | W40 | 06-shared-proto | 03-step-verify-workspace |

| W45 | config | installer/build-release.sh | release-stage the built chat binaries (not git-ls-files) | `N/A` | Update the release build so it SYNTHESIZES the built rust binaries into the release tarball rather than taking them from git ls-files: build-release.sh build mode (run on a machine with cargo) runs `cargo build --release --workspace`, then copies the resulting chat-server-rs and chat-client-rs binaries into the staged root under chat/bin/ BEFORE the copy loop. DEV-ONLY preview: a developer can build them into chat/bin/ with cargo in the nix dev shell; the shipped install.sh never builds. The skill ships TWO binaries (server + client), so chat/binaries.tsv declares one row per (target, binary); tests/test-shipped-binaries.sh is extended to allow a skill to declare multiple binaries per target (one row each), each accounted for and tested. | W40,W21 | 05-update-consumers | 10-step-ship-bin |

| W46 | config | `.gitignore` | `chat/bin binary ignore rules` | `N/A` | Add gitignore rules for the built chat binaries: ignore chat/bin/chat-server-rs and chat/bin/chat-client-rs (they stay untracked; built by the RELEASE/CI, never by the shipped install.sh), and keep src/*/target/ ignored. A tracked chat/bin/.gitkeep may be added if the directory must exist in git. | W15 | 04-remove-interpreter-tiers | 05-step-gitignore |

## Decomposition review

- [x] Every definition-of-done item maps to one or more work units.
- [x] Every known affected file and changing symbol has its own work unit.
- [x] Every work unit has exactly one goal and one step.
- [x] Each goal has 2–10 work units, or records an allowed exception.
- [x] Each step has exactly one work unit and no unnamed incidental edits.
- [x] Dependencies form an executable order with no cycle.
