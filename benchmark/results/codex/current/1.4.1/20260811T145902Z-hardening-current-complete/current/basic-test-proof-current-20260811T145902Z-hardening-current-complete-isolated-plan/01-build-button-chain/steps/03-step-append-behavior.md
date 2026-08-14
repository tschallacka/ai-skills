# Step: 03-step-append-behavior

## Ownership

- Goal: `01-build-button-chain`
- Work unit: `W03`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `appendNextButton`
- Subscope: `N/A`

## Objective

§ 4.1
Implement click behavior so only the current last button appends exactly one next button, and the fourth generated button clears the document and renders finished.

## Instructions

§ 5.1
Implement appendNextButton in button-chain.html so the current last button is the only button that appends. A click on it appends exactly one new button below the previous last button and transfers append behavior to the new last button.

§ 5.2
Maintain a generated-button counter that starts at zero for appended buttons. When the user clicks the fourth generated button, clear the document content and render one completion element with exact text finished and class .completion-message.

§ 5.3
Do not add tests or alter CSS in this step; those are owned by W02, W04, and W05.

## Acceptance criteria

§ 6.1
After each of the first four real current-last-button clicks, the future UI appends exactly one button for clicks one through four, then the fourth generated button click clears the previous document content.

§ 6.2
The terminal document content contains exact lowercase text finished and no residual chain buttons.

## Handoff

§ 7.1
W04 and W05 can verify W03 by counting buttons after each click and checking the terminal finished state.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
