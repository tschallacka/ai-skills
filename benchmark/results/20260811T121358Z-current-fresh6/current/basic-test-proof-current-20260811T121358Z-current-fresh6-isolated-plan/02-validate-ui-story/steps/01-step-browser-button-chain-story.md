# Step: 01-step-browser-button-chain-story

## Ownership

- Goal: `02-validate-ui-story`
- Work unit: `W06`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `US-01 browser button-chain flow`
- Subscope: `N/A`

## Objective

§ 4.1
Run the future browser story that clicks the current last button until completion and records pass or bug evidence.

## Instructions

§ 5.1
When executing later, open the local button-chain.html in a browser and use only direct clicks on the visible current last button.

§ 5.2
Click the initial button to create generated button one, click generated buttons one through three to append the next generated button, then click generated button four to complete.

## Acceptance criteria

§ 6.1
The story passes only if each pre-completion click appends exactly one button below the last button, pressing generated button four clears the document, and the page shows exact text finished with a visible white border.

## Handoff

§ 7.1
If the browser story fails, record a bugs.md row and create investigation and fix goals before retesting.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
