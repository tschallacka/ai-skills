# Step: 06-step-test-router

## Ownership

- Goal: `02-page-shell`
- Work unit: `W12`
- Type: `test`

## Change target

- File: `src/plan-overview/tests/router.rs`
- Primary symbol or file scope: `route_table`
- Subscope: `N/A`

## Objective

§ 4.1
Pin every route so a later page addition cannot break deep linking silently.

## Instructions

§ 5.1
Assert each page and parameter shape resolves, and that an unknown page name, a malformed hash and a non-existent id each resolve to the overview carrying the unresolved value. Fault-inject by removing one route arm and by feeding a hash with an injected separator.

## Acceptance criteria

§ 6.1
The test fails when a route arm is removed and when a malformed hash resolves to anything other than the overview with its reason recorded.

## Handoff

§ 7.1
W12 is the regression guard for W08; W48 relies on id resolution behaving as pinned here.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
