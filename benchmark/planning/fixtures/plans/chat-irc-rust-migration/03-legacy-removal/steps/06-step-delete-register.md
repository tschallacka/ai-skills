# Step: 06-step-delete-register

## Ownership

- Goal: `03-legacy-removal`
- Work unit: `W25`
- Type: `source`

## Change target

- File: `chat/scripts/chat-register.sh`
- Primary symbol or file scope: `delete legacy register helper`
- Subscope: `N/A`

## Objective

§ 4.1
git rm chat/scripts/chat-register.sh. Channel registration is implicit via JOIN on the rust server.

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
