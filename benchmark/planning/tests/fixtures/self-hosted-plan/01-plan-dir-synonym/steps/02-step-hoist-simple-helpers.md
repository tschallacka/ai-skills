# Step: 02-step-hoist-simple-helpers

## Ownership

- Goal: `01-plan-dir-synonym`
- Work unit: `W02`
- Type: `source`

## Change target

- File: `planning/scripts/add-goal.sh`
- Primary symbol or file scope: `argument parsing`
- Subscope: `N/A`

## Objective

§ 4.1
Source plan-document-lib.sh above first use of $1, then hoist --plan-dir to position 1.

## Instructions

§ 5.1
Insert the hoist after script_dir is defined and the library is sourced, in each of the twenty positional helpers.

## Acceptance criteria

§ 6.1
Every helper accepts --plan-dir and none reports an undefined function at load.

## Handoff

§ 7.1
The differential unit can compare both argument forms for real invocations.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
