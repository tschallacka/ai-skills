# Step: 03-step-finished-border-style

## Ownership

- Goal: `01-button-chain-contract`
- Work unit: `W03`
- Type: `style`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `.completion-message`
- Subscope: `N/A`

## Objective

§ 4.1
Define the completion presentation so the final lowercase text finished is visible with a white border.

## Instructions

§ 5.1
Define .completion-message in button-chain.html with styling that makes a border visibly white around the exact finished text.

§ 5.2
Keep the selector scoped to the terminal message and do not alter button-chain logic in this style step.

## Acceptance criteria

§ 6.1
An element with class completion-message displays a border whose color is visibly white.

§ 6.2
The style can be applied after the document is cleared by W04 without relying on external CSS.

## Handoff

§ 7.1
W04 can render the terminal message with class completion-message to satisfy the white-border requirement.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
