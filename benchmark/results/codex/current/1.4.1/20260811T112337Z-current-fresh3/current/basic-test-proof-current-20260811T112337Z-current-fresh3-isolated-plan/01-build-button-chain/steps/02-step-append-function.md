# Step: 02-step-append-function

## Ownership

- Goal: `01-build-button-chain`
- Work unit: `W02`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `appendGeneratedButton()`
- Subscope: `N/A`

## Objective

§ 4.1
Append exactly one new generated button immediately below the current last button each time the append path runs.

## Instructions

§ 5.1
During future execution, add appendGeneratedButton() in button-chain.html. It creates one generated button with a generated index one greater than the current generated count and inserts it directly below the current last button.

## Acceptance criteria

§ 6.1
Each call to appendGeneratedButton() increases the visible generated-button count by exactly one and preserves vertical order below the previous last button.

## Handoff

§ 7.1
W03 can call appendGeneratedButton() after confirming the clicked button is eligible to act.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
