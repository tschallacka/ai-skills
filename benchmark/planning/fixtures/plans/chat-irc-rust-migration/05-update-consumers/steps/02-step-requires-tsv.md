# Step: 02-step-requires-tsv

## Ownership

- Goal: `05-update-consumers`
- Work unit: `W33`
- Type: `config`

## Change target

- File: `chat/requires.tsv`
- Primary symbol or file scope: `requirement rows (drop interpreter/bash rows)`
- Subscope: `N/A`

## Objective

§ 4.1
Rewrite chat/requires.tsv: remove the bash-hard row and the python3/node/perl/socat soft server-runtimes group. Declare NO runtime tool requirement — the rust server mints its own cert in-crate (rcgen) and the client pins it via TOFU, so the shipped binaries need nothing at runtime.

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
