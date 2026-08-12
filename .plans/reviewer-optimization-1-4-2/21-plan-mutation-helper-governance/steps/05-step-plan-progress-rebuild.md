# Step: 05-step-plan-progress-rebuild

## Ownership

- Goal: `21-plan-mutation-helper-governance`
- Work unit: `W94`
- Type: `source`

## Change target

- File: `planning/scripts/rebuild-plan-progress.sh`
- Primary symbol or file scope: `Rebuild plan-level progress from goal trackers`
- Subscope: `N/A`

## Objective

§ 4.1
Provide atomic aggregate progress rebuild after durable mutations

## Instructions

§ 5.1
Reconstruct the plan-level progress tracker from all goal trackers, calculate the aggregate completion percentage, and replace the target atomically.

## Acceptance criteria

§ 6.1
The rebuilt tracker has one row per goal, accurate completed-goal totals, and no temporary file remains after success or failure.

## Handoff

§ 7.1
W96 can verify aggregate progress after helper-created goals and completed steps.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
