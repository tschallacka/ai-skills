# Step: 06-step-empty-approved-fixture

## Ownership

- Goal: `15-fixture-corpus`
- Work unit: `W96`
- Type: `data`

## Change target

- File: `planning/tests/fixtures/overview/empty-approved`
- Primary symbol or file scope: `approved with no steps fixture`
- Subscope: `N/A`

## Objective

§ 4.1
Create exactly one approved-with-zero-steps fixture. It is deliberately ambiguous for mode derivation and must render as a readable empty plan with no alternative fixture or escape hatch.

## Instructions

§ 5.1
Build planning/tests/fixtures/overview/empty-approved as an approved plan with no steps at all. If a helper refuses to approve a plan with no steps use the helper output only to construct the checked-in fixture and record that the refusal is not the expected rendered behavior. The fixture must retain approval and zero steps so W52 can assert ambiguous and the page can assert readable empty content. Do not provide an alternative fixture or an either-or acceptance path.

## Acceptance criteria

§ 6.1
The fixture is exactly approved with zero steps. W52 derives ambiguous with the approval and zero-step contradiction named. The page renders a readable empty plan rather than an error and US-65 observes that exact result.

## Handoff

§ 7.1
The degenerate-shape story has its state, and the page's empty-list handling is observed rather than assumed.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
