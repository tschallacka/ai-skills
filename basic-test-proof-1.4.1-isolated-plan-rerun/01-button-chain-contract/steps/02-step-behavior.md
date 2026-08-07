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
Implement the future behavior: pressing the current last button appends exactly one button below it; pressing the fourth generated button clears the document and prints finished with a white border.

## Instructions

§ 5.1
During future execution only, define appendButtonChain() and bind its current-last-button callback. Treat only appended buttons as generated. On the initial button press append generated button 1 below it; on presses of generated buttons 1, 2, and 3 append exactly one successor below and permanently retire append authority from the prior button. On the press of generated button 4, append nothing, clear the entire document, and render only exact lowercase finished using W01's visible white-border presentation. Guard each eligible activation so a rapid repeat cannot append twice.

## Acceptance criteria

§ 6.1
Static and later runtime review confirm five total presses to completion: initial creates generated 1; generated 1 creates 2; generated 2 creates 3; generated 3 creates 4; generated 4 clears and finishes. Every pre-completion eligible press adds exactly one button directly below, earlier buttons cannot add again, no fifth generated button is created, and completion leaves no prior document nodes.

## Handoff

§ 7.1
W07 receives the precise transition table and completion invariant; W03 may run only after W07 confirms the contract.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
