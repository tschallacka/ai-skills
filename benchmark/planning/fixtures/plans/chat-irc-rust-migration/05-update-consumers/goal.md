# Goal: Update skill consumers to rust-only chat

## Current state and prior-goal handoffs

§ 2.1
<confirmed facts and prerequisite handoffs>

## Outcome and definition of done

§ 3.1
Every consumer references only the rust server + rust client: install.sh skill_files() + SKILL_NAMES; chat/requires.tsv (no interpreter/bash rows); chat/SKILL.md and chat/docs/README.md (rust-only); README.md skills table; package.json files; and chat/tests/test-chat.sh rewritten to drive the rust binaries and pass. No removed file is named anywhere in the shipped skill.

## Why this goal is needed

§ 4.1
After goals 03/04 delete the files, the manifest, installer, docs and tests must agree or the skill ships broken (the installer-manifest and skill-files-manifest tests enforce this).

## Scope

§ 5.1
In: update install.sh skill_files + SKILL_NAMES, chat/requires.tsv, chat/SKILL.md, chat/docs/README.md, README.md, package.json, and rewrite chat/tests/test-chat.sh. Out: deleting source files (goals 03/04) and any functional change to the rust server/client.

## Affected files, systems, data, and interfaces

§ 6.1
install.sh, chat/requires.tsv, chat/SKILL.md, chat/docs/README.md, README.md, package.json, chat/tests/test-chat.sh. Also check coupling.tsv and tests/test-skill-files-manifest.sh for rows naming removed chat files.

## Dependencies and handoffs

§ 7.1
Depends on 03 and 04 (files deleted), plus 02 (rust client exists to be tested). No subsequent goal.

## Implementation approach, risks, and edge cases

§ 8.1
Update each consumer in one coordinated change (MAINTAINER.md 2.1): install.sh skill_files(), chat/requires.tsv rows, the two docs, README, package.json. Rewrite chat/tests/test-chat.sh to drive the rust binaries via cargo. Risk: test-skill-files-manifest.sh and test-installer-manifest.sh assert byte-consistency between tree and manifests — update deltas must be exact. Risk: removing the bash row may trip test-limited-run-contract/test-portability-contract — re-run the suite.

## Owned work units

§ 9.1
`W21` — Update the installer SOURCE — installer/src/50-manifest.sh (and installer/src/05-config.sh) — so the generated install.sh skill_files() chat block lists the prebuilt rust server and client binaries under chat/bin/ (BUILT BY THE RELEASE/CI, NEVER BY THE SHIPPED INSTALL.SH, and not committed), with NO crate-build step in skill_files() (install.sh never runs cargo: a released payload has no src/ and no cargo on the target). SKILL_NAMES/description text reflects rust-only chat. Regenerate install.sh with installer/build.sh afterwards.

§ 9.2
`W33` — Rewrite chat/requires.tsv: remove the bash-hard row and the python3/node/perl/socat soft server-runtimes group. Declare NO runtime tool requirement — the rust server mints its own cert in-crate (rcgen) and the client pins it via TOFU, so the shipped binaries need nothing at runtime.

§ 9.3
`W34` — Rewrite chat/SKILL.md: describe the rust server start, rust client send/read-delta/tail/discover commands, the additive FETCH extension, TLS + TOFU, and remove all bash-helper and interpreter-fallback guidance.

§ 9.4
`W35` — Rewrite chat/docs/README.md to present the rust server + rust client (build, run, discover, send/read/tail), dropping the bash-helper and runtime-fallbacks framing.

§ 9.5
`W36` — Update the README.md skills-table row for Chat to describe the rust server + rust client, removing the bash/runtime-fallbacks wording.

§ 9.6
`W37` — Update package.json files entry for chat: include the released chat/bin/* binaries (built by the release/CI, not committed) and drop references to the deleted scripts/ and runtime/ files. Do NOT include src/ (the binaries are prebuilt).

§ 9.7
`W38` — Rewrite chat/tests/test-chat.sh to drive only the rust server and rust client: run under `nix develop` (cargo/rustc only on PATH there), build the two binaries into chat/bin/, start the rust server, run the rust client discover/send/read-delta/tail against it, assert TLS (TOFU pinning) and FETCH delta (id > since, terminating marker), and drop all runtime/interpreter/bash expectations.

§ 9.8
`W43` — Update tests/test-portability-contract.sh allowlist rows that reference the now-deleted chat scripts and the chat python3/server-runtime groups; remove or retarget them so the portability contract passes after the removal. Also refresh PORTABILITY.md if it enumerates chat files.

§ 9.9
`W45` — Update the release build so it SYNTHESIZES the built rust binaries into the release tarball rather than taking them from git ls-files: build-release.sh build mode (run on a machine with cargo) runs `cargo build --release --workspace`, then copies the resulting chat-server-rs and chat-client-rs binaries into the staged root under chat/bin/ BEFORE the copy loop. DEV-ONLY preview: a developer can build them into chat/bin/ with cargo in the nix dev shell; the shipped install.sh never builds. The skill ships TWO binaries (server + client), so chat/binaries.tsv declares one row per (target, binary); tests/test-shipped-binaries.sh is extended to allow a skill to declare multiple binaries per target (one row each), each accounted for and tested.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | assert the manifest and installer agree with the tree, and that the rewritten chat test suite (rust server + client) passes. |
## Goal-size exception
