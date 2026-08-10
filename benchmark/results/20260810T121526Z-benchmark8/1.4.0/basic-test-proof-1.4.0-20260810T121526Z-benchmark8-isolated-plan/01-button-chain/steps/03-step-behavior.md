# Step: 03-step-behavior

## Ownership

- Goal: `01-button-chain`
- Work unit: `W03`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `button-chain click handler`
- Subscope: `N/A`

## Objective

§ 4.1
Implement the future click-handler behavior after W06 has established the initializer and state. The handler must make the initial button create generated button 1; pressing generated buttons 1–3 create generated buttons 2–4, each exactly one immediate next button below the current last button; pressing generated button 4 removes the document's button-chain root and renders the terminal state using the completion-message styling hook.

## Instructions

§ 5.1
Implement only the `button-chain click handler` in button-chain.html. Consume the initializer/state from W06 and track generated-button activations deterministically. The current target is always the final visible button in #button-chain-root, and the handler must refresh that target after every append. The initial-button activation creates generated button 1; activations on generated buttons 1, 2, and 3 create generated buttons 2, 3, and 4 respectively, each exactly one immediate next button below the current last button. The activation on generated button 4 removes the button-chain root from the document so zero buttons remain, then renders exactly one visible terminal element as the document's only application content; its text is exactly finished, its class uses .completion-message, and its contrasting background makes the explicit white border distinguishable. Do not add unrelated controls, persistence, or network behavior.

## Acceptance criteria

§ 6.1
The first four clicks result in exactly 2, 3, 4, and 5 visible buttons respectively, with generated buttons 1–4 immediately below their prior targets and adjacent order preserved. The fifth click, pressing generated button 4, removes the button-chain root, leaves zero buttons, and leaves exactly one visible terminal element containing exact lowercase finished on a contrasting background with a non-zero solid white completion-message border.

## Handoff

§ 7.1
W04 consumes the complete future HTML contract and runs US-01 from a fresh local-file context.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
