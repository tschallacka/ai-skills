# Step: 08-step-test-layout

## Ownership

- Goal: `07-graph-and-coverage-of-data`
- Work unit: `W57`
- Type: `test`

## Change target

- File: `src/plan-overview/tests/graph_layout.rs`
- Primary symbol or file scope: `layout_is_stable`
- Subscope: `N/A`

## Objective

§ 4.1
Pin determinism and minimal displacement, since both are easy to lose accidentally.

## Instructions

§ 5.1
Assert identical positions for one plan across two runs, unchanged positions when inventory rows are reordered, and bounded displacement of existing nodes when a unit is added. Fault-inject a random tiebreak and require the test to fail.

## Acceptance criteria

§ 6.1
The test fails under a random tiebreak and when adding a unit moves existing nodes beyond the stated bound.

## Handoff

§ 7.1
W57 is the guard for W54; W56 assumes both properties hold.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
