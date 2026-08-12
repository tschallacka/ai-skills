# Step: 03-step-finish-handler

## Ownership

- Goal: `01-button-chain-contract`
- Work unit: `W03`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `finishOnFourthGeneratedButton()`
- Subscope: `N/A`

## Objective

§ 4.1
Define the terminal branch so pressing the fourth generated button clears the document and prints exactly lowercase finished.

## Instructions

§ 5.1
Implement finishOnFourthGeneratedButton() as the terminal branch reached when the fourth generated button is pressed. It must clear existing document content and render one completion element whose textContent is exactly finished.

## Acceptance criteria

§ 6.1
Pressing generated buttons one through three does not finish the flow. Pressing the fourth generated button clears all buttons and leaves exactly the lowercase text finished as the user-visible content.

## Handoff

§ 7.1
W04 can style the single completion element without changing the finish text or adding other visible content.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
