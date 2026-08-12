# Step: 02-step-add-append-handler

## Ownership

- Goal: `01-create-button-chain-file`
- Work unit: `W02`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `appendNextButton()`
- Subscope: `N/A`

## Objective

§ 4.1
Add click behavior so only pressing the current last button appends exactly one new button directly below it.

## Instructions

§ 5.1
Add appendNextButton() in button-chain.html. The handler must determine whether the clicked button is the current last button; only that button may append exactly one new generated button directly below it.

§ 5.2
Increment a generated-button counter only when a button is appended. Attach the same last-button behavior to newly generated buttons without pre-creating extra buttons.

## Acceptance criteria

§ 6.1
Clicking the current last button once increases the visible button count by exactly one. Clicking an older non-last button does not append a new button.

## Handoff

§ 7.1
W03 can use the generated-button counter and current-last-button rule to identify generated button four.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
