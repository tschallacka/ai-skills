# Step: 01-step-delete-server-sh

## Ownership

- Goal: `03-legacy-removal`
- Work unit: `W20`
- Type: `source`

## Change target

- File: `chat/scripts/chat-server.sh`
- Primary symbol or file scope: `delete the legacy launcher`
- Subscope: `N/A`

## Objective

§ 4.1
git rm chat/scripts/chat-server.sh (the bash server launcher). The rust server binary is started directly by the rust client/skill; no bash launcher remains.

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
