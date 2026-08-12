# Step: 02-step-contract-tests

## Ownership

- Goal: `12-environment-contract-evolution`
- Work unit: `W71`
- Type: `test`

## Change target

- File: `planning/tests/test-plan-env.sh`
- Primary symbol or file scope: `schema rejection fixture`
- Subscope: `N/A`

## Objective

Extend manifest fixtures so an unknown or stale schema cannot be sourced.

## Instructions

Add focused assertions to `planning/tests/test-plan-env.sh` for an unknown
schema key and an unknown variable. Confirm validation fails before any
manifest content executes and preserve the no-fallback contract.

## Acceptance criteria

Unknown schema state fails closed, no sentinel command executes, and the
focused test remains deterministic.

## Handoff

Future manifest migrations must update this test with the new schema contract.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
