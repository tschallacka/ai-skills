# Step: 05-step-gitignore

## Ownership

- Goal: `04-remove-interpreter-tiers`
- Work unit: `W46`
- Type: `config`

## Change target

- File: `.gitignore`
- Primary symbol or file scope: `chat/bin binary ignore rules`
- Subscope: `N/A`

## Objective

§ 4.1
Add gitignore rules for the built chat binaries: ignore chat/bin/chat-server-rs and chat/bin/chat-client-rs (they stay untracked; built by the RELEASE/CI, never by the shipped install.sh), and keep src/*/target/ ignored. A tracked chat/bin/.gitkeep may be added if the directory must exist in git.

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
