# Step: 07-step-fresh-fixture

## Ownership

- Goal: `15-fixture-corpus`
- Work unit: `W97`
- Type: `data`

## Change target

- File: `planning/tests/fixtures/overview/fresh`
- Primary symbol or file scope: `fresh plan fixture`
- Subscope: `N/A`

## Objective

§ 4.1
A fixture with no findings, no completed steps and no review cycles, which is what a plan looks like on its first day and what the page must not present as a failure.

## Instructions

§ 5.1
Build planning/tests/fixtures/overview/fresh as a plan with goals and steps but no findings, no completed steps and no review cycles, which is what a plan looks like on the day it is created.

## Acceptance criteria

§ 6.1
The fixture has zero findings, zero completed steps and zero review cycles, and the page presents each of those as not started rather than as missing or as a failure.

## Handoff

§ 7.1
The first-day story has a state, and the difference between not started and failed is observable rather than a matter of wording.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
