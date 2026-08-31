# Step: 02-step-size-fixture

## Ownership

- Goal: `09-verification`
- Work unit: `W47`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `size fixture renders and serves`
- Subscope: `N/A`

## Objective

§ 4.1
Prove the case that fails today works, end to end.

## Instructions

§ 5.1
Render and serve the 337 KB fixture, navigate three pages, and record the artifact size, the time to first page, and the cinematic tier the page settled at.

## Acceptance criteria

§ 6.1
The fixture renders and serves, its pages navigate without an error or empty region, and the measurements are recorded as numbers. US-12 and US-53 pass.

## Handoff

§ 7.1
These numbers are the before-and-after evidence against the current exit 126.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
