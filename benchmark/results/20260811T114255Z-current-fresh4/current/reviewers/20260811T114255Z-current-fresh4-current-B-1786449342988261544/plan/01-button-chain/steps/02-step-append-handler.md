# Step: 02-step-append-handler

## Ownership

- Goal: `01-button-chain`
- Work unit: `W02`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `appendNextButton()`
- Subscope: `last-button click path`

## Objective

§ 4.1
Add the click handling that only responds when the clicked control is the current last button and appends exactly one new button below it.

## Instructions

§ 5.1
In future execution, add appendNextButton() in button-chain.html. It must determine whether the clicked button is the current last button before appending. When valid, create exactly one new button and insert it below the current last button.

§ 5.2
Track generated button count separately from the initial button so generated button one is the first appended button. Do not implement the fourth-generated completion branch in this step.

## Acceptance criteria

§ 6.1
Each accepted click on the current last button appends exactly one button below it.

§ 6.2
Clicking a button that is no longer the current last button does not append another button.

## Handoff

§ 7.1
W03 can rely on a generated-button counter and a last-button click path that appends one button per accepted interaction.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
