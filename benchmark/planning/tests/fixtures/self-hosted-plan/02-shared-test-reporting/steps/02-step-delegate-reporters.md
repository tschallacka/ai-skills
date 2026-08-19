# Step: 02-step-delegate-reporters

## Ownership

- Goal: `02-shared-test-reporting`
- Work unit: `W06`
- Type: `test`

## Change target

- File: `planning/tests/test-progress-bar-shape.sh`
- Primary symbol or file scope: `reporter definition`
- Subscope: `note_fail`

## Objective

§ 4.1
Point the local reporter at t_fail and replace the counter epilogue with t_end, leaving every message and call site unchanged.

## Instructions

§ 5.1
Point each counter-style reporter at t_record and read t_failures in its epilogue, leaving messages and call sites untouched.

## Acceptance criteria

§ 6.1
Twenty-six tests share the implementation and every message is unchanged.

## Handoff

§ 7.1
The baseline comparison can attribute any difference to this change alone.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
