# Step: 01-step-history-page

## Ownership

- Goal: `06-pages-history`
- Work unit: `W32`
- Type: `source`

## Change target

- File: `src/plan-overview/src/pages/history.rs`
- Primary symbol or file scope: `render_history()`
- Subscope: `N/A`

## Objective

§ 4.1
Show how the plan got here, which the current page discards entirely.

## Instructions

§ 5.1
Render status transitions with their times, the review cycles with their findings and outcome, and the current phase, most recent first. A transition with no recorded time is shown as undated rather than placed arbitrarily. A cycle with no findings is shown as a completed cycle rather than omitted.

## Acceptance criteria

§ 6.1
Transitions are ordered and each names the item with its old and new status; the current cycle is identifiable; an undated transition and an empty cycle are both shown as such. US-25 and US-46 apply.

## Handoff

§ 7.1
W21 draws its moved-recently list from these same transitions.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
