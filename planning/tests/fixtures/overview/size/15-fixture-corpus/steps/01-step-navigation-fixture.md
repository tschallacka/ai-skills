# Step: 01-step-navigation-fixture

## Ownership

- Goal: `15-fixture-corpus`
- Work unit: `W91`
- Type: `data`

## Change target

- File: `planning/tests/fixtures/overview/navigation`
- Primary symbol or file scope: `navigation fixture snapshot`
- Subscope: `N/A`

## Objective

§ 4.1
Check in a frozen snapshot of the 82-unit plan the navigation stories are recorded against, replacing the live plans-root directory as the evidence source. A live plan can be changed or deleted by a plans-root helper, so evidence recorded against it is not reproducible.

## Instructions

§ 5.1
Copy the 82-unit plan from the plans root into planning/tests/fixtures/overview/navigation as a frozen snapshot, and record in the fixture the source plan name, its counts and the date of the copy. Copy only; the live plan is not edited, moved or removed.

## Acceptance criteria

§ 6.1
The snapshot is in the repository, renders and serves identically to the live plan it came from on the day it was taken, and carries the recorded provenance. Deleting the live plan from the plans root leaves every navigation measurement reproducible.

## Handoff

§ 7.1
Every navigation and graph story has a fixture it can name instead of a live plan, and W99 has a fixture whose contents it can pin.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
