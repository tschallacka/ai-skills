# Step: 07-step-helper-regression-coverage

## Ownership

- Goal: `21-plan-mutation-helper-governance`
- Work unit: `W96`
- Type: `test`

## Change target

- File: `planning/tests/test-progress-helpers.sh`
- Primary symbol or file scope: `Exercise all helper-only mutation paths`
- Subscope: `N/A`

## Objective

§ 4.1
Cover companion, progress, rebuild, finding, and dispatcher behavior

## Instructions

§ 5.1
Run the focused helper regression, reviewer projection, installer manifest, plan validation, and adversarial-review status checks against isolated temporary fixtures where applicable.

## Acceptance criteria

§ 6.1
All focused tests pass, the package manifest remains synchronized, the plan validator reports no structural errors, and the helper-only review finding is resolved.

## Handoff

§ 7.1
Goal 21 is ready for completion and the plan can proceed to final user-disposition and release-gate validation.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
