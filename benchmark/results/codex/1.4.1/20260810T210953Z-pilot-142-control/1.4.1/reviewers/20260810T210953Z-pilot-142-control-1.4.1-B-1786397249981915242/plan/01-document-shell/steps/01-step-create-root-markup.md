# Step: 01-step-create-root-markup

## Ownership

- Goal: `01-document-shell`
- Work unit: `W01`
- Type: `markup`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `#button-chain-root`
- Subscope: `N/A`

## Objective

§ 4.1
Create the HTML document body with one initial button and a stable root for appended buttons.

## Instructions

§ 5.1
Create `button-chain.html` with a minimal valid HTML document. Inside `#button-chain-root`, place exactly one visible initial button and no generated buttons. Do not add click behavior in this step.

## Acceptance criteria

§ 6.1
Source inspection shows one `#button-chain-root` subtree and exactly one initial button in the initial document. No additional buttons, completion text, or generated-button markup is present at load time.

## Handoff

§ 7.1
`W02` can style the completion selector without changing root markup, and `W03` can attach behavior to the existing root and initial button.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
