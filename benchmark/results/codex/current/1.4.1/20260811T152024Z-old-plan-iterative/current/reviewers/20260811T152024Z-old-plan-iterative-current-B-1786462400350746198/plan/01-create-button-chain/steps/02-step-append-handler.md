# Step: 02-step-append-handler

## Ownership

- Goal: `01-create-button-chain`
- Work unit: `W02`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `button-chain script`
- Subscope: `append-last-button handler`

## Objective

§ 4.1
Attach click handling so pressing only the current last button appends exactly one new button below it.

## Instructions

§ 5.1
Add the click behavior for the current-last-button path in button-chain.html. The handler must append exactly one new button below the current last button and then make that new button the current last button.

## Acceptance criteria

§ 6.1
Pass when each valid current-last click before completion increases the button count by one, appends the new button below the previous last button, and does not append multiple buttons.

## Handoff

§ 7.1
W03 can rely on generated buttons being counted in append order and the current-last reference being accurate.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
