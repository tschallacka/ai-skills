# Step: 03-step-breadcrumbs

## Ownership

- Goal: `02-page-shell`
- Work unit: `W09`
- Type: `source`

## Change target

- File: `src/plan-overview/src/render/chrome.rs`
- Primary symbol or file scope: `breadcrumbs()`
- Subscope: `N/A`

## Objective

§ 4.1
Show a reader arriving by deep link where the page sits, without requiring them to have walked there.

## Instructions

§ 5.1
Render the trail for the resolved route from the plan hierarchy: plan, goal, then the item. Each level is a link. A route with no parent above the plan renders a single level rather than an empty bar.

## Acceptance criteria

§ 6.1
A deep-linked unit page shows plan, goal and unit, each navigable, on first load with no prior navigation. US-02 and US-52 both depend on this being present rather than derived from history.

## Handoff

§ 7.1
W52 uses the trail to state the current page for keyboard and landmark order.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
