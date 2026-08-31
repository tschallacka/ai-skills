# Step: 03-step-test-data-coverage

## Ownership

- Goal: `07-graph-and-coverage-of-data`
- Work unit: `W39`
- Type: `test`

## Change target

- File: `src/plan-overview/tests/data_coverage.rs`
- Primary symbol or file scope: `every_state_field_presented`
- Subscope: `N/A`

## Objective

§ 4.1
Guarantee mechanically that no stored field is extracted and then dropped.

## Instructions

§ 5.1
Enumerate the parsed field set from W02 and assert each field is consumed by at least one page renderer. Fail naming any field no page consumes. Fault-inject by adding a field to the fixture and confirming the failure names it.

## Acceptance criteria

§ 6.1
The test names an unconsumed field rather than passing silently, and the injected field causes a failure that identifies it. US-31 is the reader-facing counterpart.

## Handoff

§ 7.1
This is the guard behind the promise that all stored data is presented.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
