# Step: 04-step-profile-tests

## Ownership

- Goal: `01-contract-profile`
- Work unit: `W04`
- Type: `test`

## Change target

- File: `planning/tests/test-plan-commands.sh`
- Primary symbol or file scope: `reviewer profile contract tests`
- Subscope: `N/A`

## Objective

§ 4.1
Test marked-section extraction, missing/empty section rejection, hash/version metadata, and generated-profile drift detection.

## Instructions

§ 5.1
Work only on `planning/tests/test-plan-commands.sh`, targeting `reviewer profile contract tests`. Test marked-section extraction, missing/empty section rejection, hash/version metadata, and generated-profile drift detection. Keep unrelated files and symbols out of this step; preserve prerequisite contracts and evidence boundaries.

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
