# Step: 04-step-finished-border

## Ownership

- Goal: `01-button-chain-html`
- Work unit: `W04`
- Type: `style`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `.completion-message`
- Subscope: `N/A`

## Objective

§ 4.1
Style the completion state so the finished text has a visible white border.

## Instructions

§ 5.1
Add .completion-message styling in button-chain.html. Give the completion element a visible white border with enough padding and a contrasting page or element background so the border is plainly visible.

§ 5.2
Keep the styling local to the completion state; do not alter the append count or click behavior in this style unit.

## Acceptance criteria

§ 6.1
After completion, the exact finished text is enclosed by a visible white border.

§ 6.2
The border remains visible on a default desktop browser viewport without relying on browser devtools or source inspection.

## Handoff

§ 7.1
W05 can verify both the exact completion text and visible bordered presentation through the rendered page.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
