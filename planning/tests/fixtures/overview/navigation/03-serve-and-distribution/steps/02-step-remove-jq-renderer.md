# Step: 02-step-remove-jq-renderer

## Ownership

- Goal: `03-serve-and-distribution`
- Work unit: `W15`
- Type: `source`

## Change target

- File: `planning/scripts/render-plan-overview.sh`
- Primary symbol or file scope: `file removal`
- Subscope: `N/A`

## Objective

§ 4.1
Delete the superseded renderer so no second implementation of the page exists.

## Instructions

§ 5.1
Delete planning/scripts/render-plan-overview.sh and nothing else. An earlier version of this instruction also said to remove every reference to it in the skill, the manifest and the tests; that was the contradiction adversarial finding AR-04 recorded, because those files had no inventory row while this step's own atomicity check certifies that no other file changes. Each of them is now owned: the manifest by W81, the map by W82, the contract by W83, the documentation by W84 and the tests by W85 to W89. If a reference is found in a file none of those own, report it rather than editing it here.

## Acceptance criteria

§ 6.1
The renderer file is absent from the working tree and the binary is the only implementation of the page. References to the deleted name elsewhere are expected at this point and are not a failure of this unit: they are the work goals 13 and 14 own, and goal 13's definition of done is where their absence is demonstrated.

## Handoff

§ 7.1
Goal 13 can correct every declaration that names the renderer against a tree where it is genuinely gone, and W85 rewrites its assertions against the binary rather than against a file that still exists.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
