# Step: 03-step-anomalies-fixture

## Ownership

- Goal: `15-fixture-corpus`
- Work unit: `W93`
- Type: `data`

## Change target

- File: `planning/tests/fixtures/overview/anomalies`
- Primary symbol or file scope: `structural anomalies fixture`
- Subscope: `N/A`

## Objective

§ 4.1
A fixture carrying the structural edge cases no real plan happens to have: a deliberately orphaned work unit, a single-unit goal with a recorded size exception, and a goal whose testing requirement is no.

## Instructions

§ 5.1
Build planning/tests/fixtures/overview/anomalies with the plan helpers, then make exactly three minimal edits: leave one work unit that nothing depends on and that depends on nothing, record a single-unit goal with its size exception, and set one goal's testing requirement to no with a rationale. Record each edit and which story or unit needs it.

## Acceptance criteria

§ 6.1
The fixture validates as a plan, and each of the three anomalies is present and reachable: the orphan appears as a graph leaf, the single-unit goal carries its exception, and the testing-requirement row reads no. No fourth anomaly is present, so a failure against this fixture names one cause.

## Handoff

§ 7.1
The orphan-finding story and the goal-rendering stories have a state that actually contains what they claim to observe.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
