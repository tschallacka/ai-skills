# Step: 01-step-dom-test

## Ownership

- Goal: `02-proof-and-handoff`
- Work unit: `W05`
- Type: `test`

## Change target

- File: `button-chain.test.js`
- Primary symbol or file scope: `button-chain interaction test`
- Subscope: `N/A`

## Objective

§ 4.1
Add an automated DOM interaction test that clicks through the chain and asserts initial, append, fourth-generated clear, exact finished text, and visible white border behavior.

## Instructions

§ 5.1
Future executor adds button-chain.test.js using the repository-appropriate DOM testing harness. The test loads button-chain.html, performs real DOM click dispatches against the current last button, checks counts after each append, verifies prior buttons do not append, then clicks generated button 4 and checks the final body content and border style.

## Acceptance criteria

§ 6.1
The test fails unless the initial button count is one, clicks 1-4 append exactly one button each, click 5 on generated button 4 removes all buttons, body text is exactly finished, and the completion element has a visible white border.

## Handoff

§ 7.1
W06 can rely on the DOM test command and results as pre-browser evidence, but still must run the real browser story separately.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
