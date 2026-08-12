# Step: 02-step-package-regression-tests

## Ownership

- Goal: `18-package-regression-coverage`
- Work unit: `W83`
- Type: `test`

## Change target

- File: `planning/tests/test-installer-manifest.sh`
- Primary symbol or file scope: `package regression resolution test`
- Subscope: `N/A`

## Objective

Prove each newly packaged regression file is emitted and resolves to the
repository source.

## Instructions

Extend the installer test for the new rows, exact row count, source existence,
destination resolution, and exclusion of local `.env` files.

## Acceptance criteria

The installer test fails on package drift, missing sources, or accidental
publication of local manifests.

## Handoff

Hand off package evidence to the final review and release gate.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
