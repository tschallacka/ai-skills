# Step: 01-step-process-cancellation

## Ownership

- Goal: `06-safeguards-telemetry`
- Work unit: `W28`
- Type: `source`

## Change target

- File: `benchmark/planning/setup-benchmark.sh`
- Primary symbol or file scope: `generated worker cleanup_on_signal()`
- Subscope: `N/A`

## Objective

§ 4.1
Forward an interrupt to the worker-owned process group and descendants, wait for cleanup, remove case temporary state, and preserve interrupted evidence. Outer batch propagation is owned by the separate runner step.

## Instructions

§ 5.1
Work only on `benchmark/planning/setup-benchmark.sh`, targeting `generated worker cleanup_on_signal()`. Forward an interrupt to the worker-owned process group and descendants, wait for cleanup, remove case temporary state, and preserve interrupted evidence. Keep unrelated files and symbols out of this step; preserve prerequisite contracts and evidence boundaries.

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
