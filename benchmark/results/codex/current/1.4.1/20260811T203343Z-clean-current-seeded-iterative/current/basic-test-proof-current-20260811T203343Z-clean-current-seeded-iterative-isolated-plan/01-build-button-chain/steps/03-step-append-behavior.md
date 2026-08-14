# Step: 03-step-append-behavior

## Ownership

- Goal: `01-build-button-chain`
- Work unit: `W03`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `appendNextButton()`
- Subscope: `N/A`

## Objective

§ 4.1
Add click behavior so pressing the current last button appends exactly one new button below it and disables or ignores prior buttons as append sources.

## Instructions

§ 5.1
Implement appendNextButton() so a click on the current last button appends exactly one new generated button below it, increments a generated-button counter, and makes the newly appended button the only append-eligible current last button. Earlier buttons must not append additional buttons after they are no longer last.

## Acceptance criteria

§ 6.1
Each valid click before completion increases the visible button count by exactly one and places the new button below the previous last button. Clicking any earlier button after a new last button exists does not append another button.

## Handoff

§ 7.1
W04 can use the generated-button counter and current-last-button state to detect the fourth generated button click.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
