# Step: 01-step-tier-table

## Ownership

- Goal: `12-adaptive-cinematics`
- Work unit: `W71`
- Type: `source`

## Change target

- File: `src/plan-overview/src/render/tiers.rs`
- Primary symbol or file scope: `emit_tier_table()`
- Subscope: `N/A`

## Objective

§ 4.1
Declare the tiers once, where they can be reviewed and tested without a browser.

## Instructions

§ 5.1
Emit one table of tiers: the sustained frame-time threshold for stepping down, the threshold for stepping up, the hysteresis window, and exactly which effects each tier disables. Thresholds live here rather than inside the client so they are reviewable in one place and testable in the language the rest of the engine is written in.

## Acceptance criteria

§ 6.1
The table names three tiers with their thresholds, window and disabled effect sets, and the client contains no threshold of its own. US-84 requires the table to explain why a tier is active.

## Handoff

§ 7.1
W72 and W73 consume it; W74 tests the decision it encodes.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
