# Step: 02-step-size-fixture

## Ownership

- Goal: `15-fixture-corpus`
- Work unit: `W92`
- Type: `data`

## Change target

- File: `planning/tests/fixtures/overview/size`
- Primary symbol or file scope: `size fixture snapshot`
- Subscope: `N/A`

## Objective

§ 4.1
Check in a frozen snapshot of the 337 KB state that fails to render today, so the size claim and the memory ceiling are measured against a fixed input rather than a moving one.

## Instructions

§ 5.1
Copy the 337 KB state plan into planning/tests/fixtures/overview/size as a frozen snapshot with the same recorded provenance. Record the exact byte size of the state document in the fixture, because it is the number the size claim and the memory ceiling are both stated against.

## Acceptance criteria

§ 6.1
The snapshot is in the repository, its recorded byte size matches the file, and the failure the plan exists to fix reproduces against it rather than only against a live plan.

## Handoff

§ 7.1
W47 and W90 measure against a fixed input, so a later regression in either is attributable to the renderer rather than to the fixture having grown.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
