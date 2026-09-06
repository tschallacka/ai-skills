# Goal: Remove legacy bash clients and interpreter server tiers; update consumers

## Current state and prior-goal handoffs

§ 2.1
Handoff from 01 + 02: the rust server and rust client now implement the whole chat path. The legacy surface remains: bash helpers chat/scripts/chat-send.sh, read.sh, tail.sh, register.sh, watch.sh, announce.sh, discover.sh, server.sh; interpreter tiers chat/runtime/server.py, server.js, server.pl, bash-handler.sh; chat/requires.tsv runtime rows; install.sh skill_files() chat block + SKILL_NAMES descriptions; README.md skills table row; package.json files entry. chat/tests/test-chat.sh still exercises the removed bash tiers.

## Outcome and definition of done

§ 3.1
All bash chat helpers and interpreter server tiers are deleted. The skill ships only the rust server and rust client. Every consumer is updated: install.sh skill_files() + SKILL_NAMES + description no longer reference removed files; chat/requires.tsv no longer declares python3/node/perl/socat groups (nothing interpreter runs); README.md skills table and chat/docs/README.md describe the rust rust path only; package.json files entry stays correct; chat/tests/test-chat.sh is rewritten to drive the rust server+client and is green. No committed binary (src/*/target/ and chat/runtime/chat-server-rs stay gitignored).

## Why this goal is needed

§ 4.1
The legacy bash/interpreter path doubles maintenance, is not interoperable with standard IRC clients, and contradicts the compiled, dependency-light direction. Removing it after 01/02 fulfil the rust path avoids shipping two chat implementations.

## Scope

§ 5.1
In: deleting the bash helpers and interpreter tiers; updating install.sh, chat/requires.tsv, chat/SKILL.md, chat/docs/README.md, README.md, package.json, and chat/tests/test-chat.sh. Out: changes to the rust server/client functionality; keeping any bash tier; touching planning/ or other skills; migrating stored chat logs (they stay under AI_CHAT_HOME and remain readable via FETCH if the format is kept).

## Affected files, systems, data, and interfaces

§ 6.1
Deleted: chat/scripts/chat-send.sh, chat-read.sh, chat-tail.sh, chat-register.sh, chat-watch.sh, chat-announce.sh, chat-discover.sh, chat-server.sh; chat/runtime/server.py, server.js, server.pl, bash-handler.sh; chat/runtime/chat-server-rs (built artifact). Updated: install.sh (skill_files() chat block, SKILL_NAMES), chat/requires.tsv, chat/SKILL.md, chat/docs/README.md, README.md, package.json, chat/tests/test-chat.sh. Check coupling.tsv + tests/test-skill-files-manifest.sh for rows naming removed chat files.

## Dependencies and handoffs

§ 7.1
Depends on: 01-irc-server and 02-irc-client (the rust path must be complete and test-green before removing bash). No subsequent goal.

## Implementation approach, risks, and edge cases

§ 8.1
Approach: after 01/02 are green, git rm the bash/interpreter files, then update each consumer in the same change (MAINTAINER.md 2.1 coordinated migration). Update the installer source (installer/src/50-manifest.sh + 05-config.sh, then regenerate install.sh with installer/build.sh) so the chat block lists the two prebuilt rust binaries under chat/bin/ (BUILT BY THE RELEASE/CI, never by the shipped install.sh, not committed) and the interpreter/bash requirement rows are removed. Rewrite chat/tests/test-chat.sh to cover only rust. Risk: install.sh/README/coupling must be byte-consistent with the actual tree (test-skill-files-manifest.sh, test-installer-manifest.sh assert this). Risk: coupling.tsv or other tests reference removed files - update them. Risk: removing the chat requires.tsv bash row may trip test-limited-run-contract/test-portability-contract - re-run them.

## Owned work units

§ 9.1
`W20` — git rm chat/scripts/chat-server.sh (the bash server launcher). The rust server binary is started directly by the rust client/skill; no bash launcher remains.

§ 9.2
`W22` — git rm chat/scripts/chat-send.sh. Replaced by the rust client send command.

§ 9.3
`W23` — git rm chat/scripts/chat-read.sh. Replaced by the rust client read-delta command.

§ 9.4
`W24` — git rm chat/scripts/chat-tail.sh. Replaced by the rust client tail command.

§ 9.5
`W25` — git rm chat/scripts/chat-register.sh. Channel registration is implicit via JOIN on the rust server.

§ 9.6
`W26` — git rm chat/scripts/chat-watch.sh. Replaced by the rust client tail/poll.

§ 9.7
`W27` — git rm chat/scripts/chat-announce.sh. The rust server now broadcasts the UDP announce beacon itself.

§ 9.8
`W28` — git rm chat/scripts/chat-discover.sh. The rust client now discovers via its own UDP listener.

## Testing requirement

| Test required | Rationale |
|---|---|
| no | Pure deletion units; the rust functionality they remove was verified in goals 01 and 02. Any residual check is structural (grep for remaining references), which is a manifest/test concern owned by goal 05. |
## Goal-size exception
