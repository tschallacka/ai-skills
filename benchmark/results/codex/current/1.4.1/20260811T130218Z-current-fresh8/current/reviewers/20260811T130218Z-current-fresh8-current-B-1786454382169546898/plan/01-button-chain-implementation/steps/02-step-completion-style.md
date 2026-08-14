# Step: 02-step-completion-style

## Ownership

- Goal: `01-button-chain-implementation`
- Work unit: `W02`
- Type: `style`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `.completion-message`
- Subscope: `N/A`

## Objective

§ 4.1
Define the visible completion presentation for the exact text finished, including a visible white border.

## Instructions

§ 5.1
Add only the .completion-message style rule in button-chain.html.

§ 5.2
Style must make the final finished text visibly bordered in white and visibly distinguishable from its surroundings by using a contrasting non-white completion background or equivalent contrast treatment.

§ 5.3
Keep this step limited to presentation; it does not define click behavior or render completion content.

## Acceptance criteria

§ 6.1
A completion element using .completion-message displays a white border that remains visible because the completion state has contrasting non-white background or equivalent contrast.

§ 6.2
The style preserves exact readable text finished when W04 renders the completion element.

## Handoff

§ 7.1
W04 can apply .completion-message when replacing the document with the finished state.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
