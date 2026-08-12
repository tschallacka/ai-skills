# Step: 04-step-behavior-test

## Ownership

- Goal: `01-button-chain-behavior`
- Work unit: `W04`
- Type: `test`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `buttonChainBehaviorTest`
- Subscope: `N/A`

## Objective

§ 4.1
Define an automated test target that exercises initial state, exact single append per current-last click, ignored non-last clicks, and fourth-generated completion text.

## Instructions

§ 5.1
Define an automated behavior test target for initial state, exact single append, ignored older-button click, and fourth-generated completion.

## Acceptance criteria

§ 6.1
The test design would fail if the initial button is counted as generated, more than one button is appended per click, older buttons append, or completion text differs from finished.

## Handoff

§ 7.1
W06 can reuse the same acceptance sequence for direct browser interaction.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
