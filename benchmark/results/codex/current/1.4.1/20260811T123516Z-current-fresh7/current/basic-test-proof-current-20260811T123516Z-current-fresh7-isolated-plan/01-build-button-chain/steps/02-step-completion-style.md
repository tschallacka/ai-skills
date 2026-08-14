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
Define the completion message styling so the exact text finished has a visible white border.

## Instructions

§ 5.1
Add CSS for .completion-message that makes the completion text visibly bordered in white. The border must be visible against the chosen page background.

§ 5.2
Keep the style local to button-chain.html and avoid hiding, clipping, or transforming the exact completion text.

## Acceptance criteria

§ 6.1
Source review shows .completion-message has a white border declaration or equivalent visible white border treatment, with enough padding or layout for the border to be seen.

## Handoff

§ 7.1
W04 can render the completion element with class completion-message and rely on the visible white border styling.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
