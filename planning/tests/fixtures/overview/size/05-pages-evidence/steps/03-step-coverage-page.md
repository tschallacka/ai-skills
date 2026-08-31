# Step: 03-step-coverage-page

## Ownership

- Goal: `05-pages-evidence`
- Work unit: `W29`
- Type: `source`

## Change target

- File: `src/plan-overview/src/pages/coverage.rs`
- Primary symbol or file scope: `render_coverage()`
- Subscope: `N/A`

## Objective

§ 4.1
Present the definition-of-done mapping without the cut-off unit lists the current page shows.

## Instructions

§ 5.1
Render each required outcome beside the units that produce and prove it, every id a link, no list abbreviated. An outcome with no proving unit is flagged as uncovered rather than rendered as an ordinary row with an empty cell.

## Acceptance criteria

§ 6.1
Every id in every row is visible and clickable with no ellipsis, and an uncovered outcome is visibly flagged. US-06 and US-44 apply.

## Handoff

§ 7.1
W39 asserts the coverage fields are among those presented; W44 relies on the uncovered flag existing.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
