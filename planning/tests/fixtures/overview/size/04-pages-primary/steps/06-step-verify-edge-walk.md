# Step: 06-step-verify-edge-walk

## Ownership

- Goal: `04-pages-primary`
- Work unit: `W26`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `walk three hops by edge`
- Subscope: `N/A`

## Objective

§ 4.1
Prove in the browser that a reviewer can follow the graph without an index.

## Instructions

§ 5.1
On the 82-unit fixture, start at a unit, click to a unit it depends on, then to something that depends on that. Use no index and no back button. Record each click, the resulting page, and whether any text was clipped.

## Acceptance criteria

§ 6.1
Three hops completed by relationship links alone, breadcrumbs updating each time, and no clipped or truncated content observed. US-01 passes with recorded interaction.

## Handoff

§ 7.1
This is the story that most directly replaces the current page's failure mode; its evidence belongs in the review.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
