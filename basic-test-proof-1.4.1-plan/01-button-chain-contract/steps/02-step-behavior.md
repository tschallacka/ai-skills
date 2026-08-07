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
Future execution only: implement appendButtonChain() in button-chain.html. Treat the currently last visible button as the only active append control; each pre-completion activation appends exactly one button below it. Count newly generated buttons from one, and on activation of generated button four clear the document and render finished with a visible white border. Do not execute this instruction during the current planning proof.

## Acceptance criteria

§ 6.1
The future behavior has no duplicate append, counts the fourth generated button rather than the initial button, removes prior buttons on completion, and visibly presents finished with a white border.

## Handoff

§ 7.1
W03 receives the exact click sequence: initial button, generated button one, generated button two, generated button three, generated button four; the fourth generated-button activation is the completion event.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
