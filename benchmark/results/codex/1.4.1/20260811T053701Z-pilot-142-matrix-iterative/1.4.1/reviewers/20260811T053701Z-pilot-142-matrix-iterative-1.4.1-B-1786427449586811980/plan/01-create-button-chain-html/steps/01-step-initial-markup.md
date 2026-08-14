# Step: 01-step-initial-markup

## Ownership

- Goal: `01-create-button-chain-html`
- Work unit: `W01`
- Type: `markup`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `#button-chain-root`
- Subscope: `N/A`

## Objective

§ 4.1
Create the initial #button-chain-root page structure with one visible starting button and no pre-rendered generated buttons.

## Instructions

§ 5.1
When executing this future step, create button-chain.html with a minimal HTML document and a body containing a named #button-chain-root DOM subtree with exactly one initial visible button. Use a stable user-visible label that the browser story can target, such as Add button.

§ 5.2
Do not add generated buttons in source markup. Do not add script behavior or styling in this step beyond markup needed for the initial button target.

## Acceptance criteria

§ 6.1
button-chain.html exists after execution and its source body contains exactly one button element before runtime interaction.

§ 6.2
The initial button is visible to a browser user and is the only button present on first render.

## Handoff

§ 7.1
W02 may rely on the initial button being the first and current last button at page load.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
