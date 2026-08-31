# Step: 05-step-test-parse-and-derive

## Ownership

- Goal: `01-engine-core`
- Work unit: `W05`
- Type: `test`

## Change target

- File: `src/plan-overview/tests/state_parse.rs`
- Primary symbol or file scope: `parse_state_fixture`
- Subscope: `N/A`

## Objective

§ 4.1
Pin the production state contract: serialize W102 extraction output, parse it through W02, preserve every emitted field and value, and retain unknown-field and malformed-input failures.

## Instructions

§ 5.1
Serialize each fixture through W102's production extractor, parse that output through W02, and compare every emitted field and value with the expected typed contract. Fault-inject by adding an unknown field and truncating the serialized production document; both failures must remain explicit.

## Acceptance criteria

§ 6.1
The production extractor-to-parser round trip passes for every emitted field and value. An added field is reported as unknown and truncation reports its stopping position; no field is accepted only through a disconnected fixture document.

## Handoff

§ 7.1
This is the red-line for W02 to W04; W39 extends the same fixture for field coverage.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
