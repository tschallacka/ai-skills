# Step: 05-step-complete-fixture

## Ownership

- Goal: `15-fixture-corpus`
- Work unit: `W95`
- Type: `data`

## Change target

- File: `planning/tests/fixtures/overview/complete`
- Primary symbol or file scope: `complete plan fixture`
- Subscope: `N/A`

## Objective

§ 4.1
A fixture in which every step and every verification has passed, which is the completed state no live plan in the plans root is currently in.

## Instructions

§ 5.1
Build planning/tests/fixtures/overview/complete as a small plan in which every step and every verification has passed and the review is approved. Keep it small deliberately: it exists to exercise the completed shape, not to be another size fixture.

## Acceptance criteria

§ 6.1
Every step and verification in the fixture reads as passed, the review reads as approved, and nothing in the plan is open. The completed shape is reachable without waiting for a real plan to finish.

## Handoff

§ 7.1
The stories about a finished plan, including what autoplay does when nothing is moving, have a state that is genuinely finished.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
