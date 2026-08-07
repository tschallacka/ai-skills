# Step: 03-step-validator

## Ownership

- Goal: `02-plan-proof-and-handoff`
- Work unit: `W05`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `validate-plan.sh`
- Subscope: `N/A`

## Objective

§ 4.1
Run structural plan validation after review approval and confirm the plan directory contains no HTML or implementation artifact.

## Instructions

§ 5.1
After the disclosed adversarial review is approved, run planning/scripts/validate-plan.sh on this plan. Audit only this new directory for HTML or implementation artifacts, confirm no browser/server/driver was started by the proof, and run the required context audit and benchmark under the resource-limited helper. Do not inspect HTML anywhere.

## Acceptance criteria

§ 6.1
The validator exits zero for seven units across two goals; review status is synchronized; required UI and tracker artifacts exist; context checks pass; the new directory contains no HTML; and no prohibited process was started.

## Handoff

§ 7.1
W06 records exact commands and outcomes, deferred units, safety result, and the next future action.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
