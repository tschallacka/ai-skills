# Step: 03-step-last-button-handler

## Ownership

- Goal: `01-build-button-chain`
- Work unit: `W03`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `handleButtonClick()`
- Subscope: `last-button guard`

## Objective

§ 4.1
Ignore non-last buttons and route only the current last button click to append or completion behavior.

## Instructions

§ 5.1
During future execution, implement handleButtonClick() so it first determines the current last visible button in the chain. If the clicked button is not that last button, return without appending or finishing.

## Acceptance criteria

§ 6.1
Only a click on the current last button can change the document; earlier buttons remain inert after later buttons exist.

## Handoff

§ 7.1
W04 can add completion branching inside the same click handler without weakening the last-button guard.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
