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
<direct action on this one target>

## Acceptance criteria

§ 6.1
<observable result for this target>

## Handoff

§ 7.1
<what the next named work unit can rely on>

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
