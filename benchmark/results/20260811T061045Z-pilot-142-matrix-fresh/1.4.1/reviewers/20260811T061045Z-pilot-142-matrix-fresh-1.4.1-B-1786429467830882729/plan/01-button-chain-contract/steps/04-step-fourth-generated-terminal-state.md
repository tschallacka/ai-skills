# Step: 04-step-fourth-generated-terminal-state

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
Clear the document when the fourth generated button is pressed and render only the exact lowercase text finished using the completion style.

## Instructions

§ 5.1
Add renderFinishedState() inside button-chain.html. This function clears the document content and renders the terminal message; it does not inspect click targets or append buttons.

§ 5.2
Render the exact lowercase text finished and apply .completion-message so the visible white border is present.

## Acceptance criteria

§ 6.1
When called by W07, previous buttons are gone and the document displays the exact lowercase text finished.

§ 6.2
The terminal message has a visible white border and no extra completion text changes the required string.

## Handoff

§ 7.1
W07 can call renderFinishedState() from the fourth-generated terminal branch.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
