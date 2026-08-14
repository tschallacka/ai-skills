# Step: 05-step-behavior-test

## Ownership

- Goal: `01-button-chain-implementation`
- Work unit: `W05`
- Type: `test`

## Change target

- File: `button-chain.behavior.test.mjs`
- Primary symbol or file scope: `button chain behavior test`
- Subscope: `N/A`

## Objective

§ 4.1
Add the focused node button-chain.behavior.test.mjs automated test using built-in Node.js modules only to prove initial state, exact one-button append per last-button click, ignored non-last clicks, fourth-generated completion, exact text, visible white border, and border contrast.

## Instructions

§ 5.1
Create only button-chain.behavior.test.mjs as the automated test artifact.

§ 5.2
Run it with the exact future command node button-chain.behavior.test.mjs, using built-in Node.js modules only to load button-chain.html and exercise the embedded script in a controlled DOM-like harness defined inside the test file.

§ 5.3
Do not add packages, browser drivers, generated files, or additional harness files in this work unit; any such need must become its own new work unit before execution.

## Acceptance criteria

§ 6.1
node button-chain.behavior.test.mjs fails if the initial state is not exactly one button.

§ 6.2
The command fails if an eligible current-last click appends zero or more than one button, or if a non-last click appends anything.

§ 6.3
The command fails if completion happens on the wrong generated button, leaves old content behind, changes finished casing, lacks a visible white border, or lacks contrast that makes the white border visible.

## Handoff

§ 7.1
W06 may start after node button-chain.behavior.test.mjs passes and its output is recorded by the executor.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
