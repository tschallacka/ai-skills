# Step: 02-step-completion-style

## Ownership

- Goal: `01-build-button-chain`
- Work unit: `W02`
- Type: `style`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `.completion-message`
- Subscope: `N/A`

## Objective

§ 4.1
Define completion-state styling with a visible white border around the exact finished message.

## Instructions

§ 5.1
Add CSS for .completion-message in button-chain.html. The style must create a visible white border around the message while leaving layout legible after the document is cleared.

§ 5.2
Do not change markup structure or JavaScript behavior in this style step.

## Acceptance criteria

§ 6.1
When the future completion element uses .completion-message, the border is visibly white and surrounds the exact rendered text finished.

§ 6.2
No selector other than .completion-message is changed by this work unit unless the executor first adds a separate style work unit.

## Handoff

§ 7.1
W03 can apply .completion-message to the terminal element and W04/W05 can assert visible white border styling.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
