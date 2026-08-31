# Step: 04-step-apply-change

## Ownership

- Goal: `10-live-updates`
- Work unit: `W61`
- Type: `source`

## Change target

- File: `src/plan-overview/assets/live.js`
- Primary symbol or file scope: `applyStateChange`
- Subscope: `N/A`

## Objective

§ 4.1
Apply an update without costing the reader their place.

## Instructions

§ 5.1
Update values in place, hand the graph its before and after so it can animate rather than redraw, and preserve scroll position, expanded sections, any open peek and the selected autoplay tab. State when the page last updated.

## Acceptance criteria

§ 6.1
After an update the scroll position, expansion, open peek and selected tab are unchanged, the graph animated rather than redrew, and the last-updated time is shown. US-70 and US-66 apply.

## Handoff

§ 7.1
This is the unit that makes the whole live experience feel intentional rather than jarring, which was the original complaint about the previous page.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
