# Step: 03-step-add-terminal-state

## Ownership

- Goal: `01-create-button-chain-file`
- Work unit: `W03`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `handleTerminalGeneratedButton()`
- Subscope: `fourth generated button branch`

## Objective

§ 4.1
When the fourth generated button is pressed, clear the document and render only the completion state with exact text finished.

## Instructions

§ 5.1
Add handleTerminalGeneratedButton() for the fourth generated button branch. When generated button four is clicked as the current last button, clear the document body and render the completion message with exact text finished.

§ 5.2
Do not leave the initial button, generated buttons, hidden controls, or extra visible text in the body after the terminal state.

## Acceptance criteria

§ 6.1
After generated button four is clicked, the body contains the completion state only, and the visible text is exactly finished in lowercase.

## Handoff

§ 7.1
W04 can style the completion message element without changing its text or terminal-state creation logic.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
