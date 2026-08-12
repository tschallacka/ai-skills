# Step: 01-step-markup-root

## Ownership

- Goal: `01-build-button-chain`
- Work unit: `W01`
- Type: `markup`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `#button-chain-root`
- Subscope: `N/A`

## Objective

§ 4.1
Create the document body root containing exactly one initial button and no pre-rendered generated buttons.

## Instructions

§ 5.1
In the future implementation, create button-chain.html as a standalone HTML document. In #button-chain-root place exactly one button as the initial visible control and do not pre-render generated buttons.

§ 5.2
Give the initial button a stable accessible name and any data attributes needed by appendNextButton, but do not include styling or click behavior in this markup step.

## Acceptance criteria

§ 6.1
Opening the future document before interaction shows exactly one button inside #button-chain-root and no completion message.

§ 6.2
The markup target is limited to the root DOM subtree; CSS and JavaScript behavior remain owned by W02 and W03.

## Handoff

§ 7.1
W02 can rely on a completion message class name being available for styling, and W03 can rely on a single initial button root.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
