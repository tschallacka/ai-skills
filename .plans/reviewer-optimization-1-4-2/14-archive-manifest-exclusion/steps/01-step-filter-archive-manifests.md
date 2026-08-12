# Step: 01-step-filter-archive-manifests

## Ownership

- Goal: `14-archive-manifest-exclusion`
- Work unit: `W74`
- Type: `source`

## Change target

- File: `benchmark/planning/setup-benchmark.sh`
- Primary symbol or file scope: `publication staging copy`
- Subscope: `N/A`

## Objective

Prevent `.env` and temporary manifest files from entering result archives.

## Instructions

Change publication staging to copy the workspace while explicitly excluding
local manifest names, without filtering ordinary plan evidence or reviewer
artifacts. Keep active workspace manifests untouched.

## Acceptance criteria

Staging contains no `.env` or `.env.tmp.*` file, and the plan and required
non-secret evidence remain present.

## Handoff

W75 verifies the publication boundary.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
