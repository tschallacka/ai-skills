# Step: 04-step-test-tiers

## Ownership

- Goal: `12-adaptive-cinematics`
- Work unit: `W74`
- Type: `test`

## Change target

- File: `src/plan-overview/tests/tiers.rs`
- Primary symbol or file scope: `tier_selection`
- Subscope: `N/A`

## Objective

§ 4.1
Pin the tier decision, especially that it cannot flap.

## Instructions

§ 5.1
Assert a sustained miss steps down one tier, sustained headroom steps up one, a figure inside the hysteresis window changes nothing, and no sequence of alternating fast and slow samples produces oscillation. Fault-inject by removing the hysteresis window and requiring the alternating sequence to fail.

## Acceptance criteria

§ 6.1
All four properties pinned, and removing the hysteresis makes the alternating case fail rather than passing by chance.

## Handoff

§ 7.1
W74 guards W71 to W73; US-81 exercises recovery in a real browser.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
