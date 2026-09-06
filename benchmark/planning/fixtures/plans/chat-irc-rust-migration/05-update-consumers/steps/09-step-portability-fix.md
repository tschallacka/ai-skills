# Step: 09-step-portability-fix

## Ownership

- Goal: `05-update-consumers`
- Work unit: `W43`
- Type: `config`

## Change target

- File: `planning/tests/test-portability-contract.sh`
- Primary symbol or file scope: `allowlist rows for the removed chat scripts + python3 group`
- Subscope: `N/A`

## Objective

§ 4.1
Update tests/test-portability-contract.sh allowlist rows that reference the now-deleted chat scripts and the chat python3/server-runtime groups; remove or retarget them so the portability contract passes after the removal. Also refresh PORTABILITY.md if it enumerates chat files.

## Instructions

§ 5.1
<direct action on this one target>

## Acceptance criteria

§ 6.1
<observable result for this target>

## Handoff

§ 7.1
<what the next named work unit can rely on>

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
