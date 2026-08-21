# Step: 03-step-hoist-subcommand-helper

## Ownership

- Goal: `01-plan-dir-synonym`
- Work unit: `W03`
- Type: `source`

## Change target

- File: `planning/scripts/update-plan-content.sh`
- Primary symbol or file scope: `argument parsing`
- Subscope: `per-subcommand case`

## Objective

§ 4.1
Hoist at position 1 after command="$1"; shift, so every subcommand sees the plan directory positionally.

## Instructions

§ 5.1
Insert the hoist after update-plan-content.sh shifts its subcommand off, so the plan directory is position 1 for every form.

## Acceptance criteria

§ 6.1
The -dp, -ap and -f subcommands rewrite identical documents either way.

## Handoff

§ 7.1
The differential unit can cover the subcommand shape as well as the plain one.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
