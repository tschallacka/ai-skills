# Step: 01-step-add-chain-handler

## Ownership

- Goal: `02-chain-behavior`
- Work unit: `W03`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `handleChainClick(event)`
- Subscope: `N/A`

## Objective

§ 4.1
Implement delegated click handling so only the current last button appends one button and the fourth generated button clears the document to finished.

## Instructions

§ 5.1
Implement `handleChainClick(event)` so a click is accepted only when it targets the current last button. Accepted clicks before the terminal state append exactly one new button below the current last button. When the accepted target is the fourth generated button, replace the document content with an element containing exact lowercase text `finished` and the `.completion-state` border styling.

## Acceptance criteria

§ 6.1
Manual reasoning and later verification can confirm that the initial button creates generated button 1, generated buttons 1 through 3 each append the next generated button, and generated button 4 clears the document to only the bordered `finished` state.

## Handoff

§ 7.1
`W04` and `W05` can verify the generated-button counter, current-last-button guard, exact terminal text, clear operation, and visible white border.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
