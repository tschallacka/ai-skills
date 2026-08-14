# Step: 04-step-completion-border

## Ownership

- Goal: `01-define-button-chain-page`
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
In the future implementation, add .completion-message styling for the completion element only.

§ 5.2
Use a border color that is visibly white against the page background and make the border width and style visible without relying on browser default focus outlines.

## Acceptance criteria

§ 6.1
The finished text is enclosed or otherwise bounded by a visible white border in the completion state.

## Handoff

§ 7.1
W05 and W06 can assert both text and white-border visibility.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
