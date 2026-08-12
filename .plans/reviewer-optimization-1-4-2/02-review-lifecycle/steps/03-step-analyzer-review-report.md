# Step: 03-step-analyzer-review-report

## Ownership

- Goal: `02-review-lifecycle`
- Work unit: `W08`
- Type: `docs`

## Change target

- File: `benchmark/planning/analyzer-prompt.md`
- Primary symbol or file scope: `review lifecycle report`
- Subscope: `N/A`

## Objective

§ 4.1
Require separate reporting of review cycles, verification passes, termination/handoff events, independence status, and unresolved limits.

## Instructions

§ 5.1
Work only on `benchmark/planning/analyzer-prompt.md`, targeting `review lifecycle and independence report sections`. Require separate reporting of review cycles, verification passes, termination/handoff events, independence status, and unresolved limits. Keep unrelated files and symbols out of this step; preserve prerequisite contracts and evidence boundaries.

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
