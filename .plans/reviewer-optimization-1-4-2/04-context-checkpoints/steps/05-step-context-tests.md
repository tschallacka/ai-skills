# Step: 05-step-context-tests

## Ownership

- Goal: `04-context-checkpoints`
- Work unit: `W20`
- Type: `test`

## Change target

- File: `planning/tests/test-plan-context.sh`
- Primary symbol or file scope: `context cache test function`
- Subscope: `N/A`

## Objective

§ 4.1
Verify bounded reads, changed-entry detection, source namespace freshness, invalidation after mutation, and no shared-state collision.

## Instructions

§ 5.1
Work only on `planning/tests/test-plan-context.sh`, targeting `bounded context cache and invalidation cases`. Verify bounded reads, changed-entry detection, source namespace freshness, invalidation after mutation, and no shared-state collision. Keep unrelated files and symbols out of this step; preserve prerequisite contracts and evidence boundaries.

## Acceptance criteria

§ 6.1
The named target has the required behavior, its output is bounded and reproducible, and the companion or downstream verification can observe the result without an unnamed change.

## Handoff

§ 7.1
Record the changed target, command/output evidence, and unavailable or environment-specific results for the next dependency.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
