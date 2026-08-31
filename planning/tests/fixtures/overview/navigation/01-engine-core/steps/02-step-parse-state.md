# Step: 02-step-parse-state

## Ownership

- Goal: `01-engine-core`
- Work unit: `W02`
- Type: `source`

## Change target

- File: `src/plan-overview/src/plan/state.rs`
- Primary symbol or file scope: `parse_state()`
- Subscope: `N/A`

## Objective

§ 4.1
Parse the canonical serialized state produced by W102 into typed values, reporting unknown fields and malformed input instead of dropping or partially presenting them.

## Instructions

§ 5.1
Parse the canonical serialized state produced by W102 into typed structures for identity, goals, steps, edges, testingMarks, coverage, findings, cycles and reviewTarget. The parser remains usable for supplied fixture documents, but its primary contract test serializes W102 output and parses it back, so an emitted field cannot bypass the parser. Report unknown fields and malformed input instead of dropping or partially presenting them.

## Acceptance criteria

§ 6.1
W102's production output round-trips through this parser with every emitted field and value preserved. Every field listed above is available as a typed value, an unrecognised field appears in the reported set, and a truncated document produces an error naming where parsing stopped. US-54 depends on that error being reportable to the page.

## Handoff

§ 7.1
W03 and W50 derive from these values. W39 enumerates the parsed field set to assert every field reaches a page, so the reported unknown-field set is part of that contract.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
