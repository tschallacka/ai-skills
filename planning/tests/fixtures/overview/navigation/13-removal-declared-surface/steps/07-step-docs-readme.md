# Step: 07-step-docs-readme

## Ownership

- Goal: `13-removal-declared-surface`
- Work unit: `W84`
- Type: `docs`

## Change target

- File: `planning/docs/README.md`
- Primary symbol or file scope: `overview section`
- Subscope: `N/A`

## Objective

§ 4.1
Correct the reference documentation for the overview: how it is rendered, how it is served, and what a platform without a prebuilt artifact is told.

## Instructions

§ 5.1
In the overview section of planning/docs/README.md, rewrite how the page is rendered, how it is served, and what a platform without a matching artifact is told, using the same command names W83 put in the contract. Where the old text explained the runtime rung selection, say instead that the binary serves the artifact itself.

## Acceptance criteria

§ 6.1
The document names neither removed script, its commands match the contract word for word, and the unavailability text matches the message the installer actually prints rather than a paraphrase of it.

## Handoff

§ 7.1
Goal 13's definition of done is reachable: a clean-checkout install can be searched for a reference to the removed renderer and find none, in declarations, manifests and documents alike. Goal 14 begins against a tree where every non-test surface is already correct.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
