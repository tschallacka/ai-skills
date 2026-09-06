# Step: 07-step-rewrite-test

## Ownership

- Goal: `05-update-consumers`
- Work unit: `W38`
- Type: `test`

## Change target

- File: `chat/tests/test-chat.sh`
- Primary symbol or file scope: `rewrite to rust server + client`
- Subscope: `N/A`

## Objective

§ 4.1
Rewrite chat/tests/test-chat.sh to drive only the rust server and rust client: run under `nix develop` (cargo/rustc only on PATH there), build the two binaries into chat/bin/, start the rust server, run the rust client discover/send/read-delta/tail against it, assert TLS (TOFU pinning) and FETCH delta (id > since, terminating marker), and drop all runtime/interpreter/bash expectations.

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
