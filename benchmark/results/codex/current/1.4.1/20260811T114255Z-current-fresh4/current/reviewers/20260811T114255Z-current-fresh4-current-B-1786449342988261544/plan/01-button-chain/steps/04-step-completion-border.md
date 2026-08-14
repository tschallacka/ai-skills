# Step: 04-step-completion-border

## Ownership

- Goal: `01-button-chain`
- Work unit: `W04`
- Type: `style`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `.completion-message`
- Subscope: `N/A`

## Objective

§ 4.1
Style the completion state so the lowercase finished text has a visible white border.

## Instructions

§ 5.1
In future execution, style .completion-message in button-chain.html so the finished completion state has a visible white border. Ensure the surrounding background or spacing makes the white border observable rather than blending into a white page.

§ 5.2
Do not change append or completion branching behavior in this style step.

## Acceptance criteria

§ 6.1
The completion message has a white border that is visibly distinguishable in the rendered page.

§ 6.2
The style does not alter the exact finished text or reintroduce any cleared buttons.

## Handoff

§ 7.1
W05 can rely on the completed markup, behavior, and style targets being ready for browser verification.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
