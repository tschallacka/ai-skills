# Step: 02-step-context-views

## Ownership

- Goal: `04-context-checkpoints`
- Work unit: `W17`
- Type: `source`

## Change target

- File: `planning/scripts/plan-context.sh`
- Primary symbol or file scope: `context read/check command dispatch`
- Subscope: `N/A`

## Objective

§ 4.1
Expose bounded phase-specific summary, ownership/dependency, changed-document, and validator-focused views with fixed command contracts and size limits.

## Instructions

§ 5.1
Work only on `planning/scripts/plan-context.sh`, targeting `context read/check command dispatch`. Expose bounded phase-specific summary, ownership/dependency, changed-document, and validator-focused views with fixed command contracts and size limits. Keep unrelated files and symbols out of this step; preserve prerequisite contracts and evidence boundaries.

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
