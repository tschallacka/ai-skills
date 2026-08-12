# Step: 01-step-protocol-cohort

## Ownership

- Goal: `05-protocol-archive`
- Work unit: `W21`
- Type: `source`

## Change target

- File: `benchmark/planning/run-benchmark.sh`
- Primary symbol or file scope: `select_latest_tags()`
- Subscope: `N/A`

## Objective

§ 4.1
Start protocol 1.4.2 as a distinct cohort, preserve legacy results unchanged, and record protocol, reviewer mode, and access-control mode in run metadata.

## Instructions

§ 5.1
Work only on `benchmark/planning/run-benchmark.sh`, targeting `tag selection and run metadata`. Start protocol 1.4.2 as a distinct cohort, preserve legacy results unchanged, and record protocol, reviewer mode, and access-control mode in run metadata. Keep unrelated files and symbols out of this step; preserve prerequisite contracts and evidence boundaries.

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
