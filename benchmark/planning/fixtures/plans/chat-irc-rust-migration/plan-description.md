# Plan: IRC-compliant TLS Rust chat server & client; remove legacy bash/interpreter chat

## Current state

§ 2.1
The chat skill (CHAT 1.4.2) ships a server and clients. The server is `src/chat-server-rs` (compiled Rust, builds clean under the pinned 1.97 toolchain from flake.nix; clippy -D warnings clean). It speaks a CUSTOM wire protocol: NICK / JOIN [#chan [since]] / LEAVE / PRIVMSG #chan :text / FETCH #chan <since> / PING / QUIT, replying with bare "OK ..."/"ERR ..." and a log line `MSG <chan> <id> <ts> <nick> :<text>`. It binds AI_CHAT_BIND (default 127.0.0.1), has NO TLS, and does NOT announce over UDP. UDP announce/discover live in bash (chat/scripts/chat-announce.sh, chat-discover.sh; JSON beacon {"proto":"ai-chat/1",...} on UDP 7780). The clients are pure-bash (chat-send/read/tail/register/watch.sh). The other server runtimes are the interpreter tiers chat/runtime/server.{py,js,pl} + bash-handler.sh. There is NO rust chat client (src/tony-the-pony is an unrelated tool-call gate). CRITICAL build facts: install.sh IS A GENERATED ARTIFACT (assembled by installer/build.sh from installer/src/*.sh) — it must be edited only through installer/src/*.sh and regenerated, never hand-edited. src/ is NOT in package.json files and is NOT git-ls-files-collected by build-release.sh, but the CHAT PATH ships prebuilt binaries: the rust binaries are BUILT BY THE RELEASE/CI into chat/bin/ and carried by the release payload; install.sh never builds them (no src/, no cargo on the target). The repo toolchain (cargo/rustc) is only on PATH inside `nix develop`; run-tests.sh does not enter nix develop, so a test that invokes cargo must run from the dev shell. TLS crates available: rustls 0.23 (ring backend, proven by device-agent), rustls-pki-types, ring, and rcgen 0.13 (ring+pem) for in-crate cert minting — so the server needs NO external binary. rustls 0.23 defaults to the aws-lc-rs backend unless default-features=false, features=[ring, std, tls12] is set.

## Desired outcome

§ 3.1
By the end: (1) src/ is a cargo workspace; (2) src/chat-server-rs is an RFC-1459-grammar IRC server over TLS (rustls, ring backend) with an additive FETCH history extension and a UDP announce beacon; (3) a NEW src/chat-client-rs connects over TLS, discovers via UDP beacon, pins the cert (TOFU), and can send / read-delta / tail; (4) the legacy bash helpers and interpreter tiers are removed, and every consumer (installer/src/*.sh + regenerated install.sh, chat/requires.tsv, chat/SKILL.md, chat/docs/README.md, README.md, package.json, tests) is updated so the skill SHIPS the rust server + client as prebuilt binaries under chat/bin/ (built by the release/CI, never at install); (5) chat/tests/test-chat.sh is rewritten to drive the rust binaries and the standard-client proof is a real TLS IRC client or a faithful byte fixture plus openssl s_client -verify_quiet; (6) all crates build clippy-clean and the repo suite passes. Definition of done: a standard TLS IRC client can register/join/message; rust client discovery+send+delta+tail work; the released package carries the two prebuilt binaries.

## Approach

§ 4.1
1) Build the RFC-grammar + history-extension IRC protocol as shared types/modules so server and client agree. 2) Rework `src/chat-server-rs` to that protocol with a TLS listener and a bounded-concurrency accept loop, plus a UDP announce task. 3) Add `src/chat-client-rs`, mirroring the server crate conventions (zero-heavy deps, pinned toolchain, MODE markers, clippy gate), connecting via TLS and doing UDP discovery with TOFU cert pinning. 4) Rewrite `chat/tests/test-chat.sh` to drive rust server + client. 5) Remove the legacy bash helpers and interpreter tiers and update every consumer/manifests/tests. Other runtimes and federated/multi-server discovery are OUT of scope. THE STANDARD-CLIENT REQUIREMENT IS A PRIMARY DESIGN DRIVER: the server MUST be connectable by a third-party IRC client that supports TLS (irssi, WeeChat, HexChat, mIRC), so it MUST implement RFC 1459 message grammar and the standard registration/numeric lifecycle a stock client expects — receive NICK+USER, answer 001/002/003/004/005 (ISUPPORT), MOTD via 375/372/376, 433 on nick-in-use, 353/366 on NAMES/JOIN, and emit `:nick!user@host COMMAND param :trailing` prefix form with PRIVMSG/NOTICE and PING/PONG. The custom history-extension command (FETCH #chan <since>) stays ADDITIVE and non-conflicting with a standard client which never sends it. Verification MUST include connecting a real standard TLS IRC client (or a faithful protocol fixture asserting the full numeric sequence and server prefix form) proving register/join/message, not merely that our rust client speaks to the server.

## Scope

§ 5.1
In: the rust server (RFC grammar over TLS + announce), the new rust client, the shared chat-proto crate + src/ workspace, the removal of the bash client helpers and interpreter server tiers, the shipping path (build-release.sh builds the two crates and synthesizes prebuilt chat/bin binaries; install.sh copies them and never builds), and the update of chat/requires.tsv, chat/SKILL.md, chat/docs/README.md, README.md, package.json, tests, and the test-portability-contract allowlist. Out (explicitly excluded): keeping the interpreter/bash tiers as fallback; federation across multiple servers; IRC-oper features beyond RFC-1459 basics (PART/NAMES/TOPIC/MODE ops, op management, DCC, IRCv3 caps, SASL); any backwards-compat aliases for old custom verbs (clean break); a GUI/full interactive client; routing secrets over chat; committing the binaries or building them at install time.

## Affected areas

§ 6.1
Server: src/chat-server-rs/{Cargo.toml,rust-toolchain.toml,src/main.rs}. Client (new): src/chat-client-rs/{...}. Shared protocol lib (new): src/chat-proto/{Cargo.toml,src/*.rs}, plus a new src/Cargo.toml workspace. Updated consumers (via installer/src/*.sh then regenerate): installer/src/05-config.sh, installer/src/50-manifest.sh, then generated install.sh (MUST NOT hand-edit install.sh). Also: chat/requires.tsv, chat/SKILL.md, chat/docs/README.md, README.md, package.json, chat/tests/test-chat.sh, tests/test-portability-contract.sh allowlist rows (deleted chat scripts + python3 group), PORTABILITY.md. Removed: chat/scripts/chat-*.sh, chat/runtime/server.{py,js,pl}, bash-handler.sh, chat/runtime/chat-server-rs. Cargo deps to ADD: rustls 0.23 (ring backend), rustls-pki-types, ring, rcgen 0.13 (ring+pem features, for in-crate cert minting). NO openssl CLI at runtime.

## Constraints and decisions

§ 7.1
Conventions: crates follow CODE-STYLE.md (MODE: DEV / PACKAGE: PROD), rust-development-guidelines.md (pinned toolchain via rust-toolchain.toml, musl preferred for Linux, zero-heavy deps), and clippy --all-targets -- -D warnings. NEW: src/ becomes a cargo workspace with a shared chat-proto crate so server/client share one protocol type (this is the single-source-of-truth decision). TLS: cert generated at server first-run in-crate via rcgen into AI_CHAT_HOME (server.crt/server.key); NO regeneration if the files exist (TOFU stability). rustls uses the ring backend (default-features=false, features=[ring,std,tls12]). Client TOFU pins the server cert fingerprint (NOT webpki-roots). Clean break: no backwards-compat for old custom verbs. install.sh is generated — edit installer/src/*.sh and run installer/build.sh; never hand-edit install.sh.

## Risks and open questions

§ 8.1
Risk: RFC numeric/prefix form must match real stock clients (CAP LS, ISUPPORT 005, 353/366 trailing) — the byte fixture + a real standard TLS IRC client; CAP LS/CAP END must be answered or not rejected. Risk: cert generation via rcgen requires the crate to resolve and pin the ring backend (network available; verified). Risk: cargo/rustc are only on PATH in `nix develop` — the RELEASE/CI build and the rewritten test-chat.sh must run in the dev shell; the shipped install.sh never needs cargo. Risk: off-by-one on FETCH semantics (id > since) and the terminating marker — pin one and specify the marker byte-exactly. Risk: rustls defaulting to aws-lc-rs — pin ring via default-features=false. Risk: removing bash rows trips test-portability-contract and test-limited-run-contract — update the allowlist and re-run.

## Environment facts

§ 9.1
Verification runs local under `nix develop` from /home/tschallacka/git/ai-skills (pinned toolchain 1.97 from flake.nix; cargo/rustc are ONLY on PATH inside the dev shell). The server binds 127.0.0.1 on an ephemeral or fixed high port; the UDP announce/discover probe uses loopback (--bcast 127.0.0.1, beacon port 7780). TLS is self-signed, minted in-crate via rcgen at first run; the client pins via TOFU. The standard-client proof connects a real TLS IRC client (openssl s_client -verify_quiet) to 127.0.0.1:<port> and asserts the numeric/prefix sequence. Order: (1) src workspace + chat-proto, (2) server, (3) client, (4) test rewrite, (5) legacy removal + consumer updates + installer regen.

## Approach decisions

§ 10.1
Approach decisions (from the design + adversarial review, corrected): (a) RFC-grammar + additive history extension, NOT strict RFC 1459 — delta reads are the core agent-chat value, FETCH is additive and never sent by a standard client. (b) Self-signed cert + client TOFU pin — the cert is GENERATED AT FIRST RUN in-crate via the rcgen crate (ring-backed), removing the earlier openssl-CLI fallback that the first review proposed before confirming rcgen is usable and network-reachable; the server needs NO external binary and NO runtime tool. (c) rustls with the ring backend: default-features=false, features=["ring","std","tls12"] — rustls 0.23 defaults to aws-lc-rs otherwise. (d) Protocol types live ONCE so server and client cannot drift: a cargo workspace with a shared src/chat-proto lib crate (new src/Cargo.toml workspace housing chat-server-rs, chat-client-rs, chat-proto). (e) UDP announce moves into the rust server, discovery into the rust client. (f) THE SHIPPING MODEL: the rust binaries are BUILT BY THE RELEASE/CI (build-release.sh runs cargo build --release for the two crates and synthesizes the prebuilt binaries into chat/bin/) and are NEVER built by the shipped install.sh and NEVER committed. install.sh only copies the prebuilt chat/bin binaries from the release payload; it never runs cargo. Because install.sh is generated, the chat block is edited in installer/src/50-manifest.sh + installer/src/05-config.sh and install.sh REGENERATED via installer/build.sh. Alternatives rejected: strict RFC with no history (loses monitor); IRCv3 caps (draft, heavier); the openssl CLI cert path (an external runtime binary with a PATH dependency — superseded by rcgen); editing install.sh directly (generated file); committing binaries (giant, drift-prone).

## UI classification

- UI affected: no
- Rationale: No browser/UI. All changes are a TCP/TLS RFC-grammar chat server, a rust client, and removal of bash clients and interpreter server tiers.

## Adversarial review

- Artifact: `adversarial-review.md`
- Status: ✅ approved
