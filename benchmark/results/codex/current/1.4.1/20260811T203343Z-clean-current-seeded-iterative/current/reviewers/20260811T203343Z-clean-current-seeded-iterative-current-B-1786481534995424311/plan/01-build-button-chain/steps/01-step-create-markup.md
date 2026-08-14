# Step: 01-step-create-markup

## Ownership

- Goal: `01-build-button-chain`
- Work unit: `W01`
- Type: `markup`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `#button-chain-app`
- Subscope: `N/A`

## Objective

§ 4.1
Create the single initial-button DOM subtree and completion container target in button-chain.html.

## Instructions

§ 5.1
Create button-chain.html with a minimal valid HTML document. Inside #button-chain-app render exactly one initial visible button and no generated buttons at load time. Include a dedicated location or factory target that W04 can use for the finished completion state without adding extra initial visible content.

## Acceptance criteria

§ 6.1
button-chain.html load state contains exactly one visible button under #button-chain-app, no pre-rendered generated buttons, no visible finished text, and no unrelated page controls.

## Handoff

§ 7.1
W02 can style the planned completion element/class; W03 can attach append behavior to the initial button and generated successors.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
