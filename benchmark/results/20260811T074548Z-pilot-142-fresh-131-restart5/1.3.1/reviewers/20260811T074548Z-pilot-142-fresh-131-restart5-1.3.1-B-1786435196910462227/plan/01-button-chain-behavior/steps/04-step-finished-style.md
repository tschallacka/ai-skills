# Step: 04-step-finished-style

## Ownership

- Goal: `01-button-chain-behavior`
- Work unit: `W04`
- Type: `style`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `.completion-message`
- Subscope: `N/A`

## Objective

§ 4.1
Style the finished text so the exact lowercase word is visible with a visible white border.

## Instructions

§ 5.1
Add CSS for `.completion-message` in `button-chain.html`. Ensure the completion text remains visible and has a clearly visible white border, such as a solid white border with enough padding or contrast for visual confirmation.

## Acceptance criteria

§ 6.1
The rendered completion state shows `finished` with a visible white border. The border must be white in computed style and not hidden by zero width, transparent color, or matching background.

## Handoff

§ 7.1
W06 can assert both exact text and a visible white border on `.completion-message`.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
