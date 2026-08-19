# Step: 04-step-mutation-spotcheck

## Ownership

- Goal: `02-shared-test-reporting`
- Work unit: `W08`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `failure reporting`
- Subscope: `N/A`

## Objective

§ 4.1
A planted failure in a converted test still names its finding and still exits non-zero.

## Instructions

§ 5.1
Plant a finding after the reporter definition in a sample of converted tests, and revert t_record to a counter.

## Acceptance criteria

§ 6.1
Each planted finding is reported and exits non-zero; reverting the seam fails the permanent assertion.

## Handoff

§ 7.1
The goal is complete and the property is guarded.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
