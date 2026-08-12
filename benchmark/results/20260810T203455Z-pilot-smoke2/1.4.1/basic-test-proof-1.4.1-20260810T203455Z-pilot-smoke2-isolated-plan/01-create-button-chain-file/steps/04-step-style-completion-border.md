# Step: 04-step-style-completion-border

## Ownership

- Goal: `01-create-button-chain-file`
- Work unit: `W04`
- Type: `style`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `.completion-message`
- Subscope: `N/A`

## Objective

§ 4.1
Style the completion state with a visible white border while preserving the exact lowercase text finished.

## Instructions

§ 5.1
Add .completion-message styling in button-chain.html so the terminal completion element has a visible white border. Use enough border width/style and layout contrast for visual verification.

§ 5.2
Do not change the completion text while styling; the text must remain exactly finished.

## Acceptance criteria

§ 6.1
The terminal completion message visibly shows a white border around the exact text finished.

## Handoff

§ 7.1
W05 can verify both the exact text and visible white border in the browser.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
