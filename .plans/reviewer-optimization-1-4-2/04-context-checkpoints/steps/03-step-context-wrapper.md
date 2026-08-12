# Step: 03-step-context-wrapper

## Ownership

- Goal: `04-context-checkpoints`
- Work unit: `W18`
- Type: `source`

## Change target

- File: `benchmark/planning/setup-benchmark.sh`
- Primary symbol or file scope: `benchmark-env.sh generation`
- Subscope: `N/A`

## Objective

§ 4.1
Generate isolated variables and a wrapper that sources them before invoking plan-context.sh, avoiding shared mutable variable files.

## Instructions

§ 5.1
Work only on `benchmark/planning/setup-benchmark.sh`, targeting `per-worker .bm-vars and plan-context wrapper generation`. Generate isolated variables and a wrapper that sources them before invoking plan-context.sh, avoiding shared mutable variable files. Keep unrelated files and symbols out of this step; preserve prerequisite contracts and evidence boundaries.

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
