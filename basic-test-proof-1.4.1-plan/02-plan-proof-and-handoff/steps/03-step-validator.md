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
Run the structural plan validator after review approval and confirm the plan contains no HTML, browser, server, or implementation artifact.

## Instructions

§ 5.1
After the fresh adversarial review is approved, run bash planning/scripts/validate-plan.sh on this plan directory. Separately inspect the plan directory for button-chain.html, other HTML files, browser/server processes, and implementation artifacts; report the exact result. Do not create or test any of them.

## Acceptance criteria

§ 6.1
The normal validator exits successfully; the approved review and synchronized plan status are present; all required progress and handoff artifacts exist; and the no-artifact/process check reports no prohibited implementation artifact or leftover process.

## Handoff

§ 7.1
The final report names the plan directory, validator command and exit status, review findings and verdict, timestamps and elapsed seconds, token-cost evidence status, no-artifact result, and the next executor's deferred browser/HTML actions.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
