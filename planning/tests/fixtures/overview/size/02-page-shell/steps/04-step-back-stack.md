# Step: 04-step-back-stack

## Ownership

- Goal: `02-page-shell`
- Work unit: `W10`
- Type: `source`

## Change target

- File: `src/plan-overview/assets/nav.js`
- Primary symbol or file scope: `backStack`
- Subscope: `N/A`

## Objective

§ 4.1
Make both the browser back button and an in-page back control return to the previous page rather than the previous scroll position.

## Instructions

§ 5.1
Maintain a stack of visited routes across hash navigation, and expose an in-page back control that pops it. Restore the scroll position and expanded sections belonging to the popped route rather than leaving the reader at the top.

## Acceptance criteria

§ 6.1
Browser back and the in-page control each return to the previous page with its scroll and expansion restored; a deep link with no history still offers a sensible destination rather than a dead control. US-02 exercises both paths.

## Handoff

§ 7.1
W11 must not push a peek onto this stack; W61 preserves the restored state across a live update.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
