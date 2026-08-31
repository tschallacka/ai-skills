# Step: 04-step-unit-edges

## Ownership

- Goal: `04-pages-primary`
- Work unit: `W24`
- Type: `source`

## Change target

- File: `src/plan-overview/src/pages/unit.rs`
- Primary symbol or file scope: `render_unit_edges()`
- Subscope: `N/A`

## Objective

§ 4.1
Make relationships the navigation, which is the change that fixes the current page most directly.

## Instructions

§ 5.1
Render the unit dependencies and dependents as two distinguishable link sets, plus the verification unit that grades it. An empty set says so rather than rendering an empty list. Every id is a link that resolves, and each destination presents its own edges so traversal continues.

## Acceptance criteria

§ 6.1
Both directions are present and distinguishable, an absent relation is stated, and every link resolves to a real route. US-01, US-21 and US-38 apply, including a unit nothing verifies saying so.

## Handoff

§ 7.1
W37 renders the same edge set graphically; the two read from one derivation so they cannot disagree.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
