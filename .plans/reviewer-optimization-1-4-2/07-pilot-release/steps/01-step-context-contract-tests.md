# Step: 01-step-context-contract-tests

## Ownership

- Goal: `07-pilot-release`
- Work unit: `W34`
- Type: `test`

## Change target

- File: `planning/tests/test-planning-context-v27-contract.sh`
- Primary symbol or file scope: `1.4.2 context contract matrix`
- Subscope: `N/A`

## Objective

§ 4.1
Extend the v27 oracle/benchmark contract tests for capsule variables, source namespaces, checkpoint invalidation, bounded retry, and compact-read behavior.

## Instructions

§ 5.1
Work only on `planning/tests/test-planning-context-v27-contract.sh`, targeting `1.4.2 context contract matrix`. Extend the v27 oracle/benchmark contract tests for capsule variables, source namespaces, checkpoint invalidation, bounded retry, and compact-read behavior. Keep unrelated files and symbols out of this step; preserve prerequisite contracts and evidence boundaries.

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
