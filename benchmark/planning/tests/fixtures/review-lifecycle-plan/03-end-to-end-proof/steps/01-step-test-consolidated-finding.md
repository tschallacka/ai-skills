# Step: 01-step-test-consolidated-finding

## Ownership

- Goal: `03-end-to-end-proof`
- Work unit: `W08`
- Type: `test`

## Change target

- File: `benchmark/planning/tests/test-review-oracle.sh`
- Primary symbol or file scope: `blinded semantic oracle integration fixture`
- Subscope: `N/A`

## Objective

§ 4.1
Extend the fixture to prove one complete consolidated AR finding catches all three seeded defects and that the published report remains redacted.

## Instructions

§ 5.1
Extend the existing semantic fixture with the complete AR-01 envelope that states the contradictory top-level requirement and required correction for all three seeded signals. Assert redaction remains intact.

## Acceptance criteria

§ 6.1
The direct test passes only when true positives=3, false negatives=0, independent catches=3, exact seeded IDs remain zero, and private material is absent from public JSON.

## Handoff

§ 7.1
W09 reuses the same envelope expectations at the setup/lifecycle boundary.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
