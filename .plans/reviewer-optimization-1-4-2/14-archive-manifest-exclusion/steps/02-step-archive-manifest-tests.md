# Step: 02-step-archive-manifest-tests

## Ownership

- Goal: `14-archive-manifest-exclusion`
- Work unit: `W75`
- Type: `test`

## Change target

- File: `benchmark/planning/tests/test-archive-integrity.sh`
- Primary symbol or file scope: `publication manifest exclusion fixture`
- Subscope: `N/A`

## Objective

Prove published staging excludes local manifests and retains required evidence.

## Instructions

Add a deterministic isolated fixture and assertions for root and nested plan
`.env` files, temporary manifest files, plan evidence, and reviewer artifacts.

## Acceptance criteria

The test fails if any local manifest is published or if required plan evidence
is removed.

## Handoff

Hand off archive exclusion evidence to the release gate.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
