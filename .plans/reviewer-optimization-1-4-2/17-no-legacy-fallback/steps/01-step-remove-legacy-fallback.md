# Step: 01-step-remove-legacy-fallback

## Ownership

- Goal: `17-no-legacy-fallback`
- Work unit: `W80`
- Type: `source`

## Change target

- File: `benchmark/planning/setup-benchmark.sh`
- Primary symbol or file scope: `context fixture compatibility block`
- Subscope: `N/A`

## Objective

Eliminate active fallback and skip behavior for required context fixtures.

## Instructions

Require `PLANNING_CONTEXT_CACHE` and fail with explicit configuration or
fixture errors. Do not use another home directory or silently continue.

## Acceptance criteria

Active benchmark setup has no fallback or skip path for the required fixture;
frozen archives remain untouched.

## Handoff

W81 verifies strict behavior.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
