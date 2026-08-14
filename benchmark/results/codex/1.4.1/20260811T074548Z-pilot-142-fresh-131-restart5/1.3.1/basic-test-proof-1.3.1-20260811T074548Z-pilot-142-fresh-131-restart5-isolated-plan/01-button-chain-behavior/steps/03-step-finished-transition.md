# Step: 03-step-finished-transition

## Ownership

- Goal: `01-button-chain-behavior`
- Work unit: `W03`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `handleButtonClick(event)`
- Subscope: `fourth-generated branch`

## Objective

§ 4.1
When the fourth generated button is pressed, clear the document and render only the completion state.

## Instructions

§ 5.1
In the fourth-generated branch of `handleButtonClick(event)`, detect the click on generated button number four. Clear the document body or the full root content and render a single completion element whose text content is exactly `finished`.

## Acceptance criteria

§ 6.1
After generated button 4 is clicked, no buttons remain visible and the only user-facing completion text is exactly `finished` in lowercase.

## Handoff

§ 7.1
W04 can rely on a completion element dedicated to the finished state and carrying the `.completion-message` selector.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
