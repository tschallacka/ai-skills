# Goal: Deliver rjq for local runs and reconcile its test consumers

## Current state and prior-goal handoffs

§ 2.1
After goal 01 untracks the blob, no unit delivers rjq to a local machine: register tests call rjq bare (tests/test-register-schemas.sh), CI builds its own, and no repo document says how a developer gets one. Prior handoff: goal 01's gitignored planning/bin/<triple> path is where the binary belongs.

## Outcome and definition of done

§ 3.1
A documented one-command dev bootstrap builds src/rjq into the gitignored planning/bin/<triple> path and prepends it (bootstrap.sh), run-tests.sh invokes it when rjq is absent, and tests/test-register-schemas.sh fails with that instruction instead of a bare command-not-found while tests/test-rjq-active-references.sh's existing message is confirmed. Demonstrable: on a clean checkout without rjq on PATH, the bootstrap makes the register tests pass; removing rjq makes them fail with the named instruction.

## Why this goal is needed

§ 4.1
Without this goal, risk (3) in the plan description is a promise with no mechanism: the suite fails with a bare command-not-found and the documented bootstrap does not exist.

## Scope

§ 5.1
In: bootstrap.sh (new repo-root script, rjq arm), the register-schemas guard, and the active-references message confirmation. Out: install-time download support (T70), cargo install wrappers, and any change to the register helpers themselves.

## Affected files, systems, data, and interfaces

§ 6.1
bootstrap.sh (new); tests/test-register-schemas.sh; tests/test-rjq-active-references.sh (wording only if its message does not name the fix).

## Dependencies and handoffs

§ 7.1
Depends on goal 01's gitignore section. Handoffs: W08's run-tests bootstrap invokes bootstrap.sh when rjq is absent; W38's failure message names bootstrap.sh and T70; W31's clean-checkout run is the proof the chain works from nothing.

## Implementation approach, risks, and edge cases

§ 8.1
Triple detection mirrors the installer's uname matching so the binary lands where prepend_bundled_rjq and the installer already look. Edge: no cargo means exit 69 with the T70 release note - the no-toolchain path is named, never silent.

## Owned work units

§ 9.1
`W37` — New repo-root bootstrap.sh: when rjq is neither on PATH nor at planning/bin/<triple>/rjq, build src/rjq (cargo build --release) into the gitignored planning/bin/<triple>/ path the installer's prepend logic already knows, print the PATH line to export, and exit non-zero if cargo is missing with a message naming the T70 release download as the no-toolchain alternative.

§ 9.2
`W38` — Add a command -v rjq guard at the top that fails with the bootstrap.sh instruction and the T70 release note instead of a bare command-not-found; confirm tests/test-rjq-active-references.sh's existing unavailability message names the same fix.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | W38 is a test unit (rjq availability guard) and W37's criteria are exercised through it end to end. |

## Goal-size exception
