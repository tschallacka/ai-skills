# Step: 05-step-peek

## Ownership

- Goal: `02-page-shell`
- Work unit: `W11`
- Type: `source`

## Change target

- File: `src/plan-overview/assets/nav.js`
- Primary symbol or file scope: `peek`
- Subscope: `N/A`

## Objective

§ 4.1
Let a reader check a related item without leaving the page they are reading.

## Instructions

§ 5.1
Expand a related item inline from any relationship link without changing the route or the back-stack. Escape closes it and returns focus to the invoking link. Decide and record whether a peek inside a peek nests or is refused, and state the refusal on screen if refused.

## Acceptance criteria

§ 6.1
Opening and closing a peek leaves the route and scroll position unchanged, focus returns to the invoking link, and the nested case behaves as recorded rather than ambiguously. US-03 and US-29 both apply.

## Handoff

§ 7.1
W61 preserves an open peek across a live state update.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
