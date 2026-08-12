# Step: 03-step-fourth-generated-finish

## Ownership

- Goal: `01-button-chain-behavior`
- Work unit: `W03`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `finishOnFourthGeneratedButton`
- Subscope: `N/A`

## Objective

§ 4.1
Add the branch that clears the document when the fourth generated button is pressed and prints exactly finished.

## Instructions

§ 5.1
Implement finishOnFourthGeneratedButton so pressing the fourth appended button clears the existing document content and renders only the completion state text finished.

## Acceptance criteria

§ 6.1
The fourth generated button triggers a cleared document with exact lowercase finished; earlier generated buttons do not clear the document.

## Handoff

§ 7.1
W05 can style the completion element and W04 can assert completion behavior.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
