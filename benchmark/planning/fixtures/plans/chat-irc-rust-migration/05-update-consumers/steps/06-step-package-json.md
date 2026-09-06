# Step: 06-step-package-json

## Ownership

- Goal: `05-update-consumers`
- Work unit: `W37`
- Type: `config`

## Change target

- File: `package.json`
- Primary symbol or file scope: `files entry`
- Subscope: `N/A`

## Objective

§ 4.1
Update package.json files entry for chat: include the released chat/bin/* binaries (built by the release/CI, not committed) and drop references to the deleted scripts/ and runtime/ files. Do NOT include src/ (the binaries are prebuilt).

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
