# Step: 02-step-consumer-interface-tests

## Ownership

- Goal: `16-manifest-consumer-interface`
- Work unit: `W79`
- Type: `test`

## Change target

- File: `planning/tests/test-plan-env.sh`
- Primary symbol or file scope: `validated consumer fixture`
- Subscope: `N/A`

## Objective

Prove a temporary trusted helper uses the single documented interface.

## Instructions

Exercise valid loading, missing manifests, stale schemas, malformed values,
and bounded output. Confirm the helper never sources before validation.

## Acceptance criteria

The helper loads expected variables only through the documented interface and
all invalid cases fail closed.

## Handoff

Hand off consumer evidence to the archive and final-review goals.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
