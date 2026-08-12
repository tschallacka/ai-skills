# Step: 02-step-no-fallback-tests

## Ownership

- Goal: `17-no-legacy-fallback`
- Work unit: `W81`
- Type: `test`

## Change target

- File: `benchmark/planning/tests/test-safeguards.sh`
- Primary symbol or file scope: `strict fixture configuration test`
- Subscope: `N/A`

## Objective

Prevent fallback and silent skip behavior from returning.

## Instructions

Add assertions for unset, missing, and valid fixture configuration and ensure
the benchmark status cannot become successful when required evidence is absent.

## Acceptance criteria

The test detects fallback paths, rejects missing fixtures, and accepts only a
valid explicitly configured fixture.

## Handoff

Hand off fail-closed evidence to pilot release validation.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
