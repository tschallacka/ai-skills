# Step: 01-step-dom-regression

## Ownership

- Goal: `02-prove-button-chain-behavior`
- Work unit: `W05`
- Type: `test`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `button chain DOM regression test`
- Subscope: `N/A`

## Objective

§ 4.1
Add a planned DOM-level regression check for initial button count, append count, last-button-only behavior, and completion text.

## Instructions

§ 5.1
In the future implementation, add a bounded DOM-level regression check for button-chain.html using the repository's available lightweight HTML test approach or a documented script local to the file if no test framework exists.

§ 5.2
Assert initial one-button state, one-button append per current-last-button click, no append from stale non-last buttons, document clear on fourth generated button click, exact finished text, and the completion border style contract.

## Acceptance criteria

§ 6.1
The regression fails for missing initial button, extra appended buttons, stale-button append, missing clear, nonexact finished text, or missing visible white border contract.

## Handoff

§ 7.1
W06 can run the browser story after automated regression passes.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
