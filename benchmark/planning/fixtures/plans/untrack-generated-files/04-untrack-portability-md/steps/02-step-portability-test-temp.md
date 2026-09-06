# Step: 02-step-portability-test-temp

## Ownership

- Goal: `04-untrack-portability-md`
- Work unit: `W26`
- Type: `test`

## Change target

- File: `planning/tests/test-portability-contract.sh`
- Primary symbol or file scope: `freshness arm`
- Subscope: `N/A`

## Objective

§ 4.1
Rewrite the freshness arm: regenerate to PORTABILITY_OUTPUT temp paths and byte-compare two fresh runs, keeping the marker-id hygiene and banned-construct sweeps running against the regenerated text and the UNCONFIGURED-without-rjq behaviour unchanged.

## Instructions

§ 5.1
Rewrite the freshness arm of planning/tests/test-portability-contract.sh: regenerate to two PORTABILITY_OUTPUT temp paths and byte-compare them (stamp excluded, matching the generator's own comparison rule); the marker-id hygiene and banned-construct sweeps run unchanged against the regenerated text; the UNCONFIGURED-without-rjq behaviour is preserved exactly (report UNCONFIGURED, not failure). Fault-inject: add a bogus PORTABILITY() marker id to a script and confirm the sweep fails.

## Acceptance criteria

§ 6.1
The test passes with PORTABILITY.md absent from the tree; the injected bogus marker id fails it; without rjq it reports UNCONFIGURED as today; no assertion reads a committed PORTABILITY.md.

## Handoff

§ 7.1
W28's coupling flip relies on this test carrying the drift detection.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
