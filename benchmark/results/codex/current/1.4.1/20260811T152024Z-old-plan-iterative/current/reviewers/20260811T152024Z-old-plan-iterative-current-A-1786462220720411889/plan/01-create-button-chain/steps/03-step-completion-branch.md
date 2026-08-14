# Step: 03-step-completion-branch

## Ownership

- Goal: `01-create-button-chain`
- Work unit: `W03`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `button-chain script`
- Subscope: `fourth-generated completion branch`

## Objective

§ 4.1
When the fourth generated button is pressed, clear the document and render only the completion state.

## Instructions

§ 5.1
Extend the script so the click on generated button 4 clears the document body and renders the completion state instead of appending another button.

## Acceptance criteria

§ 6.1
Pass when generated buttons 1 through 3 continue the chain and clicking generated button 4 removes prior document content and shows only the completion view.

## Handoff

§ 7.1
W04 can rely on a completion element being rendered by the completion branch.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
