# Goal: Remove the interpreter server tiers and bash launcher

## Current state and prior-goal handoffs

§ 2.1
<confirmed facts and prerequisite handoffs>

## Outcome and definition of done

§ 3.1
chat/runtime/server.py, server.js, server.pl and bash-handler.sh are gone. The only server is the rust binary. Definition of done: those four files are removed and nothing in install.sh/tests still references them.

## Why this goal is needed

§ 4.1
<how this goal contributes to the initiative>

## Scope

§ 5.1
In: deleting server.py/js/pl + bash-handler.sh. Out: deleting any bash client helper (goal 03) or updating consumers (goal 05).

## Affected files, systems, data, and interfaces

§ 6.1
Deleted: chat/runtime/server.py, server.js, server.pl, bash-handler.sh.

## Dependencies and handoffs

§ 7.1
Depends on 01-irc-server. Handoff to 05-update-consumers.

## Implementation approach, risks, and edge cases

§ 8.1
git rm the four interpreter files after 01 is green. Risk: test-chat.sh and install.sh still reference them; those updates land in goal 05.

## Owned work units

§ 9.1
`W29` — git rm chat/runtime/server.py.

§ 9.2
`W30` — git rm chat/runtime/server.js.

§ 9.3
`W31` — git rm chat/runtime/server.pl.

§ 9.4
`W32` — git rm chat/runtime/bash-handler.sh.

§ 9.5
`W46` — Add gitignore rules for the built chat binaries: ignore chat/bin/chat-server-rs and chat/bin/chat-client-rs (they stay untracked; built by the RELEASE/CI, never by the shipped install.sh), and keep src/*/target/ ignored. A tracked chat/bin/.gitkeep may be added if the directory must exist in git.

## Testing requirement

| Test required | Rationale |
|---|---|
| no | Deletion verified structurally by goal-05 tests (test-skill-files-manifest.sh / grep). |

## Goal-size exception
