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
Implement the future button-chain behavior: pressing the current last button appends exactly one button below it, and pressing the fourth generated button clears the document and prints finished with a white border.

## Instructions

§ 5.1
Future execution only: implement appendButtonChain() in button-chain.html. Pressing the currently last visible button appends exactly one new button directly below it. Count newly appended buttons from one. Pressing appended button four clears the entire document and prints finished within a visible white border. Do not execute this instruction now.

## Acceptance criteria

§ 6.1
The future behavior appends once per valid press, only from the current last button, preserves vertical order through appended button four, counts the initial button separately, removes all prior content at completion, and visibly prints finished with a white border.

## Handoff

§ 7.1
W07 and W03 use this exact press order: initial, generated one, generated two, generated three, generated four; pressing generated four completes the flow.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
