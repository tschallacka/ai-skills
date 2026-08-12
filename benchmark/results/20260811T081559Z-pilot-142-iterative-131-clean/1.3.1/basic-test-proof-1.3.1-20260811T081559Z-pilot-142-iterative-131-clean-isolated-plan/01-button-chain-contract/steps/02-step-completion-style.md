# Step: 02-step-completion-style

## Ownership

- Goal: `01-button-chain-contract`
- Work unit: `W02`
- Type: `style`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `.completion-message`
- Subscope: `N/A`

## Objective

§ 4.1
Define the completion message presentation with a visible white border.

## Instructions

§ 5.1
In button-chain.html, define the .completion-message selector so the final message has a visible white border. Keep this work unit limited to presentation for the completion message and do not add click behavior here.

## Acceptance criteria

§ 6.1
The .completion-message selector includes a white border value that is visible against the chosen page background, with enough padding or spacing for the border to be seen around the finished text.

## Handoff

§ 7.1
W04 can use class completion-message when rendering the final state.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
