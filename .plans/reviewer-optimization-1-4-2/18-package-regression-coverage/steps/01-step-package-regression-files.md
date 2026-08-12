# Step: 01-step-package-regression-files

## Ownership

- Goal: `18-package-regression-coverage`
- Work unit: `W82`
- Type: `source`

## Change target

- File: `planning/V27-PACKAGE-MANIFEST.txt`
- Primary symbol or file scope: `regression package rows`
- Subscope: `N/A`

## Objective

Include the new contract regression files in the finite package boundary.

## Instructions

Add the intended tests and any required helper source to both package records
with matching ownership and destination rows. Do not add benchmark results or
local manifests.

## Acceptance criteria

The package records, installer output, and source tree agree exactly.

## Handoff

W83 verifies emission and resolution.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
