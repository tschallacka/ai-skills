# Step: 05-step-completion-style

## Ownership

- Goal: `01-build-button-chain`
- Work unit: `W05`
- Type: `style`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `.completion-state`
- Subscope: `N/A`

## Objective

§ 4.1
Style the completion state with a visible white border while preserving the exact text supplied by the completion branch.

## Instructions

§ 5.1
During future execution, style .completion-state with a visible white border against the page background. Do not create or change the semantic completion text in this style work unit.

## Acceptance criteria

§ 6.1
The final visible completion element keeps the exact text finished from W04 and has a visibly white border supplied by .completion-state.

## Handoff

§ 7.1
W06 can use the exact text from W04 and white border from W05 as separate browser assertions.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
