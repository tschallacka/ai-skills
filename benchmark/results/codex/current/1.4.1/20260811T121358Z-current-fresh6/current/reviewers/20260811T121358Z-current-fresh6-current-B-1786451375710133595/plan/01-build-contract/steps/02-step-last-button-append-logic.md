# Step: 02-step-last-button-append-logic

## Ownership

- Goal: `01-build-contract`
- Work unit: `W02`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `handleButtonClick`
- Subscope: `N/A`

## Objective

§ 4.1
Add the click handler that appends exactly one generated button below the current last button and ignores non-last buttons for append behavior.

## Instructions

§ 5.1
When executing later, implement handleButtonClick so it first confirms the clicked button is the current last button in the chain.

§ 5.2
For eligible clicks before completion, append exactly one new generated button immediately below the previous last button and make that new button the only append-eligible last button.

## Acceptance criteria

§ 6.1
Each eligible click increases the button count by one, positions the new button below the previous last button, and earlier buttons do not append extra buttons.

## Handoff

§ 7.1
W03 can rely on a generated-button counter that excludes the initial button.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
