# Step: 02-step-test-page

## Ownership

- Goal: `05-pages-evidence`
- Work unit: `W28`
- Type: `source`

## Change target

- File: `src/plan-overview/src/pages/tests.rs`
- Primary symbol or file scope: `render_test()`
- Subscope: `N/A`

## Objective

§ 4.1
Make tests visible, replacing twenty identical see-companion rows that are dead ends.

## Instructions

§ 5.1
Render what the test actually runs: the procedure from its testing companion, the registered command with its when-context, its status distinguishing not run from failed from passed, and the unit it proves as a link. A test-first unit states its red baseline as the expected first outcome.

## Acceptance criteria

§ 6.1
The procedure is on the page rather than referenced, the words see companion appear nowhere, an unregistered command literal is flagged rather than shown as registered, and an untested unit is not presented as proof. US-04, US-22, US-42 and US-43 apply.

## Handoff

§ 7.1
W29 links coverage rows to these pages; W31 verifies a reader can follow the procedure without leaving.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
