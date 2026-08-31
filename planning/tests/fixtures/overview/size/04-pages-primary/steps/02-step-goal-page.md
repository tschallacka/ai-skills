# Step: 02-step-goal-page

## Ownership

- Goal: `04-pages-primary`
- Work unit: `W22`
- Type: `source`

## Change target

- File: `src/plan-overview/src/pages/goal.rs`
- Primary symbol or file scope: `render_goal()`
- Subscope: `N/A`

## Objective

§ 4.1
Show one goal in full, since the current page both truncates goal names and dumps all goals as prose.

## Instructions

§ 5.1
Render the goal outcome, scope, affected areas, dependencies and handoffs, testing requirement with its rationale, any goal-size exception with its cited reason, and its owned units as links. No goal name is abbreviated anywhere, including in lists that mention it.

## Acceptance criteria

§ 6.1
Every section of the goal document is present and readable, a declared no for testing shows its rationale, a single-unit goal shows its exception, and the owned units are all links. US-14, US-23, US-36, US-59 and US-60 apply.

## Handoff

§ 7.1
W22 is the destination for the per-goal bars on the overview and for breadcrumb level two.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
