# Step: 01-step-overview-page

## Ownership

- Goal: `04-pages-primary`
- Work unit: `W21`
- Type: `source`

## Change target

- File: `src/plan-overview/src/pages/overview.rs`
- Primary symbol or file scope: `render_overview()`
- Subscope: `N/A`

## Objective

§ 4.1
Give the monitor role one page that answers what is happening now, with every number linking to its explanation.

## Instructions

§ 5.1
Render the current phase, what moved since the previous state, what is blocked, and the derived dashboard values. Every number is a link to the page enumerating what it counts. Blockers name both the waiting item and what it waits on. In planning mode the leading content is soundness rather than progress, per the mode selection.

## Acceptance criteria

§ 6.1
Each displayed number navigates to its enumeration and that enumeration sums to the number shown; each blocker offers both endpoints in one click; no goal name is truncated. US-11, US-30, US-34 and US-35 apply.

## Handoff

§ 7.1
W32 supplies the moved-recently entries from the same transitions the history page renders, so the two cannot disagree.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
