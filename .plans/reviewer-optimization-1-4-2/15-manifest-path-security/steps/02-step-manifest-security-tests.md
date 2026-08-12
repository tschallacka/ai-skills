# Step: 02-step-manifest-security-tests

## Ownership

- Goal: `15-manifest-path-security`
- Work unit: `W77`
- Type: `test`

## Change target

- File: `planning/tests/test-plan-env.sh`
- Primary symbol or file scope: `manifest security fixtures`
- Subscope: `N/A`

## Objective

Test every newly identified validator boundary.

## Instructions

Add fixtures for foreign derived paths, mismatched local roots, duplicate
assignments, expansion syntax, ownership failure, and symlinked directories.
Use sentinels to prove rejected content never executes.

## Acceptance criteria

All unsafe fixtures fail closed and valid manifests continue to load exactly
the expected variables.

## Handoff

Hand off security evidence to the final review and release validation.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
