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
Run the historical structural plan validator after review approval and confirm the plan contains no HTML, browser, server, or implementation artifact.

## Instructions

§ 5.1
After the draft is complete and the sequential adversarial review is approved, run /tmp/basic-proof-planning-1.3.0.vA8jpU/planning/scripts/validate-plan.sh against this exact plan directory. Separately audit the repository for HTML/HTM artifacts and browser/server/driver processes without starting any such process. Record command exit status and exact validator output for W05/W06.

## Acceptance criteria

§ 6.1
The historical validator exits 0 and reports the work-unit/goal counts; the repository contains no proof-created HTML artifact and no process started by this proof remains.

## Handoff

§ 7.1
W06 receives the exact validator output, artifact audit, process audit, review result, and final status.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
