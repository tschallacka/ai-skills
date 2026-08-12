# Step: 03-step-append-last-button

## Ownership

- Goal: `01-button-chain-implementation`
- Work unit: `W03`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `appendButtonAfterLastClick`
- Subscope: `N/A`

## Objective

§ 4.1
Handle clicks so only the current last button appends exactly one new button directly below it.

## Instructions

§ 5.1
Implement appendButtonAfterLastClick so a click is eligible only when the clicked button is the current last button in #button-chain-root.

§ 5.2
For each eligible non-completion click, append exactly one new button immediately below the previous last button and update generated-button state once.

§ 5.3
A click on any earlier button must leave the button count unchanged.

## Acceptance criteria

§ 6.1
Clicking the current last button before completion appends exactly one button below it.

§ 6.2
Clicking a non-last button after generated buttons exist appends zero buttons.

§ 6.3
This step does not clear the document or render finished; W04 owns that branch.

## Handoff

§ 7.1
W04 can rely on an accurate generated-button count and last-button eligibility guard.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
