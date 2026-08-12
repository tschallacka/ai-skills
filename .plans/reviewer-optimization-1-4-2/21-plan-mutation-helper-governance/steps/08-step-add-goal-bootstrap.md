# Step: 08-step-add-goal-bootstrap

## Ownership

- Goal: `21-plan-mutation-helper-governance`
- Work unit: `W97`
- Type: `source`

## Change target

- File: `planning/scripts/add-goal.sh`
- Primary symbol or file scope: `Create-plan progress bootstrap`
- Subscope: `N/A`

## Objective

§ 4.1
Create missing plan progress through the canonical helper before aggregate rebuild

## Instructions

§ 5.1
Ensure add-goal creates the canonical plan progress tracker through create-plan-progress when a new plan has none, then rebuild it atomically.

## Acceptance criteria

§ 6.1
A new plan can receive its first goal without a missing-progress error, and the resulting tracker contains the new goal exactly once.

## Handoff

§ 7.1
W99 verifies the bootstrap path and the complete plan-command lifecycle.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
