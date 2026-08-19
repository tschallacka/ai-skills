# Step: 01-step-confirm-reporter

## Ownership

- Goal: `02-shared-test-reporting`
- Work unit: `W05`
- Type: `source`

## Change target

- File: `planning/tests/lib-test.sh`
- Primary symbol or file scope: `t_fail and t_end`
- Subscope: `N/A`

## Objective

§ 4.1
No change; the shared reporter already exists and is the seam the conversion targets.

## Instructions

§ 5.1
Add t_record and t_failures to lib-test.sh, recording findings in a file rather than a counter.

## Acceptance criteria

§ 6.1
A finding raised inside a command substitution is still counted afterwards.

## Handoff

§ 7.1
The conversion unit has a seam that preserves each test message.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
