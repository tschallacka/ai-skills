# Step: 05-step-add-adapter-injection-seam

## Ownership

- Goal: `03-end-to-end-proof`
- Work unit: `W16`
- Type: `source`

## Change target

- File: `benchmark/planning/setup-benchmark.sh`
- Primary symbol or file scope: `reviewer_command_injection`
- Subscope: `N/A`

## Objective

§ 4.1
Add a test-only command/input injection seam so the adapter integration fixture can execute deterministically without invoking live codex or changing production defaults.

## Instructions

§ 5.1
Add narrowly scoped test-only overrides at the setup adapter boundary: `REVIEWER_COMMAND`, `REVIEWER_SESSION_ID`, `REVIEWER_CAPSULE_ID`, `REVIEWER_MODE`, and `REVIEWER_APPROVED_AT`. When set, execute the command with the same capsule prompt, workspace, and output capture arguments, and write/echo those exact identity values into the lifecycle event, capsule manifest, approval handoff, and reviewer transcript; when unset, retain the current generated IDs and `codex -a never exec ...` command byte-for-byte. Document that the seam is accepted only in the isolated test harness and cannot bypass Reviewer B identity, envelope validation, semantic grading, redaction, or fail-closed gates.

## Acceptance criteria

§ 6.1
The adapter can be invoked by W14 with deterministic fake command and identity inputs, and the resulting approval/lifecycle/capsule artifacts contain the same exact values for W15; an invocation without the overrides uses the existing generated identity and production command path unchanged.

## Handoff

§ 7.1
W14 has a concrete deterministic invocation seam and can assert actual adapter outputs rather than static source text or a direct oracle substitute.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
