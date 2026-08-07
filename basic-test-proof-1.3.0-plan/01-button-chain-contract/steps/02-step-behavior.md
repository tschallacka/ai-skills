# Step: 02-step-behavior

## Ownership

- Goal: `01-button-chain-contract`
- Work unit: `W02`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `appendButtonChain()`
- Subscope: `current-last-button callback`

## Objective

§ 4.1
Implement the future button-chain behavior: each current last-button press appends exactly one button below it, and the fourth generated-button press clears the document and prints finished with a white border.

## Instructions

§ 5.1
In the future execution, implement appendButtonChain() in button-chain.html so the currently last button owns the next click, each activation appends exactly one button immediately below it, and the fourth newly appended button activation clears the document and prints finished with a visible white border. Define the generated-button count explicitly as activations 1 through 4 after the initial button. Do not edit the file now.

## Acceptance criteria

§ 6.1
The future behavior has one new button after each pre-completion current-last-button click, no duplicate append from one click, and a fourth-generated-button terminal state with no prior chain remaining, finished visible, and a white border visible.

## Handoff

§ 7.1
W07 reviews the counting and terminal-state semantics; W03 later verifies the rendered behavior through direct clicks.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
