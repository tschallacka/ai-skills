# Step: 03-step-diff-against-baseline

## Ownership

- Goal: `02-shared-test-reporting`
- Work unit: `W07`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `suite output`
- Subscope: `N/A`

## Objective

§ 4.1
Every test stdout, stderr and exit status is byte-identical to the captured baseline on the passing path.

## Instructions

§ 5.1
Re-capture every test stdout, stderr and exit status and compare against the baseline with mktemp paths normalised.

## Acceptance criteria

§ 6.1
No test differs on the passing path.

## Handoff

§ 7.1
The mutation unit can assume the passing path is unchanged.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
