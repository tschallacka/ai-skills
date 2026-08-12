# Step: 06-step-outer-cancellation

## Ownership

- Goal: `02-review-lifecycle`
- Work unit: `W39`
- Type: `source`

## Change target

- File: `benchmark/planning/setup-and-run.sh`
- Primary symbol or file scope: `top-level signal trap`
- Subscope: `N/A`

## Objective

§ 4.1
Forward Ctrl+C/TERM to every active worker, reviewer, analyzer, and child process; wait for cleanup, remove temporary state, and return a distinct interrupted status.

## Instructions

§ 5.1
Work only on `benchmark/planning/setup-and-run.sh`, targeting `top-level signal traps and run cancellation`. Forward Ctrl+C/TERM to every active worker, reviewer, analyzer, and child process; wait for cleanup, remove temporary state, and return a distinct interrupted status. Keep unrelated files and symbols out of this step; preserve prerequisite contracts and evidence boundaries.

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
