# Step: 04-step-completion-style

## Ownership

- Goal: `01-button-chain-plan`
- Work unit: `W04`
- Type: `style`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `.completion-message`
- Subscope: `N/A`

## Objective

§ 4.1
Give the finished text a visible white border while preserving the exact lowercase text.

## Instructions

§ 5.1
Define only the .completion-message style in button-chain.html so the terminal element visibly renders the exact lowercase text finished with a white border against the chosen page background.

## Acceptance criteria

§ 6.1
The completion state visibly contains exactly finished, with a white border that is distinguishable in the rendered UI and no residual button chain.

## Handoff

§ 7.1
W05 can assert exact text and visible border from the rendered completion state.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
