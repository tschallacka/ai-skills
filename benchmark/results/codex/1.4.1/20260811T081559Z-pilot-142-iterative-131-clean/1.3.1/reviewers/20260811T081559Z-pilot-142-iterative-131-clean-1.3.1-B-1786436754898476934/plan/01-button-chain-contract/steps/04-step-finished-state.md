# Step: 04-step-finished-state

## Ownership

- Goal: `01-button-chain-contract`
- Work unit: `W04`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `renderFinishedState()`
- Subscope: `N/A`

## Objective

§ 4.1
When the fourth generated button is pressed, clear the document and render only the exact lowercase text finished using the completion-message styling.

## Instructions

§ 5.1
In renderFinishedState(), clear the document body or the complete app root so prior buttons are removed. Render only the exact lowercase text finished in an element using .completion-message when handleButtonClick(event) receives a click on the fourth generated button.

## Acceptance criteria

§ 6.1
After the fourth generated button is pressed, no buttons remain, the visible text is exactly finished in lowercase, and the message uses the visible white-border styling from W02.

## Handoff

§ 7.1
W05 can verify the full path by clicking the initial button and then generated buttons 1 through 4, expecting completion only after generated button 4.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
