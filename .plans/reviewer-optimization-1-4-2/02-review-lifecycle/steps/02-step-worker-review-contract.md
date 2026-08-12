# Step: 02-step-worker-review-contract

## Ownership

- Goal: `02-review-lifecycle`
- Work unit: `W07`
- Type: `docs`

## Change target

- File: `benchmark/planning/worker-prompt.md`
- Primary symbol or file scope: `iterative-review-mode instructions`
- Subscope: `N/A`

## Objective

§ 4.1
Specify Reviewer A finding ownership, bounded verification passes, concise handoff/termination, and fresh reviewer replacement rules while preserving fresh-review default behavior.

## Instructions

§ 5.1
Work only on `benchmark/planning/worker-prompt.md`, targeting `iterative-review-mode instructions`. Specify Reviewer A finding ownership, bounded verification passes, concise handoff/termination, and fresh reviewer replacement rules while preserving fresh-review default behavior. Keep unrelated files and symbols out of this step; preserve prerequisite contracts and evidence boundaries.

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
