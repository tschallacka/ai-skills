# Step: 05-step-ui-verification

## Ownership

- Goal: `01-button-chain-plan`
- Work unit: `W05`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `US-01 button-chain browser flow`
- Subscope: `N/A`

## Objective

§ 4.1
Verify initial count, one-at-a-time append behavior, fourth-button clearing, exact finished text, and visible white border through direct UI input.

## Instructions

§ 5.1
In a future browser run, navigate to the local button-chain.html route and use only rendered mouse clicks or keyboard activation: record the initial one-button state, activate the current last button four times while checking one new button after each pre-terminal press, then inspect the terminal rendered state.

## Acceptance criteria

§ 6.1
Pass only if direct UI input demonstrates the required counts and order, fourth activation clears all buttons, the visible text is exactly finished in lowercase, and the completion state has a visible white border. This benchmark records the flow as excluded from execution due to the explicit no-browser instruction.

## Handoff

§ 7.1
The future executor records browser evidence in US-01 and its run cache; this planning proof records the constraint and does not claim a browser pass.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
