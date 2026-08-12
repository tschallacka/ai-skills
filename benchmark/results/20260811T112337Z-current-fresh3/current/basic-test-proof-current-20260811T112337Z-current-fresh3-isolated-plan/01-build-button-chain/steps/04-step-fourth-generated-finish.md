# Step: 04-step-fourth-generated-finish

## Ownership

- Goal: `01-build-button-chain`
- Work unit: `W04`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `handleButtonClick()`
- Subscope: `fourth-generated completion branch`

## Objective

§ 4.1
When the clicked current last button is the fourth generated button, clear the document body instead of appending another button.

## Instructions

§ 5.1
During future execution, extend handleButtonClick() so a click on generated button 4 clears the document body and renders a completion element whose text content is exactly finished. This branch must not call appendGeneratedButton(); generated buttons 1 through 3 continue to append exactly one next button.

## Acceptance criteria

§ 6.1
After the sequence initial, generated 1, generated 2, generated 3, generated 4 is clicked, no chain buttons remain, no generated button 5 is appended, and the completion element text is exactly finished.

## Handoff

§ 7.1
W05 can style the completion element and must preserve the exact finished text rendered by this branch.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
