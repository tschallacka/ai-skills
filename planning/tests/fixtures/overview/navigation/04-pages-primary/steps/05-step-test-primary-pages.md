# Step: 05-step-test-primary-pages

## Ownership

- Goal: `04-pages-primary`
- Work unit: `W25`
- Type: `test`

## Change target

- File: `src/plan-overview/tests/pages_primary.rs`
- Primary symbol or file scope: `primary_pages_render`
- Subscope: `N/A`

## Objective

§ 4.1
Pin that each primary page renders its required fields and that no edge points nowhere.

## Instructions

§ 5.1
Assert the required fields for the overview, goal and unit pages, and that every rendered edge target resolves to a route. Fault-inject a dangling dependency id, a goal with no units and a unit with no verification.

## Acceptance criteria

§ 6.1
The test fails when an edge target does not resolve and when a required field is absent; the empty cases render their stated absence rather than an empty element.

## Handoff

§ 7.1
W26 exercises the same paths in the browser; this test is the fast guard.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
