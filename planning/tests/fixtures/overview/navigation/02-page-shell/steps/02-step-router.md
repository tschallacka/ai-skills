# Step: 02-step-router

## Ownership

- Goal: `02-page-shell`
- Work unit: `W08`
- Type: `source`

## Change target

- File: `src/plan-overview/src/render/router.rs`
- Primary symbol or file scope: `route()`
- Subscope: `N/A`

## Objective

§ 4.1
Map a URL hash to exactly one page so deep links and the back-stack have a single source of truth.

## Instructions

§ 5.1
Resolve a hash to one of overview, goal, unit, finding, test, coverage, history or graph plus its parameters. An unknown page name or an unresolvable id resolves to the overview and records the hash it could not resolve, so the page can say why.

## Acceptance criteria

§ 6.1
Each page and parameter combination resolves; an unknown hash and a non-existent unit id both resolve to the overview carrying the unresolved hash. US-15 requires the reason to be stated on the page, not silently redirected.

## Handoff

§ 7.1
W09 renders breadcrumbs from the resolved route; W10 pushes it onto the back-stack; W48 searches by id through it.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
