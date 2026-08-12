# Step: 01-step-dom-verification

## Ownership

- Goal: `02-verify-button-chain`
- Work unit: `W04`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `DOM behavior command`
- Subscope: `N/A`

## Objective

§ 4.1
Run a bounded automated verification that loads button-chain.html, performs click-equivalent checks through the DOM event path, and asserts append count, clearing, exact text, and white border style.

## Instructions

§ 5.1
After future implementation, run a bounded local command that loads button-chain.html without a server and dispatches normal click events against the current last button. Assert there is one initial button, each current-last click appends exactly one button until the terminal branch, and prior buttons do not append after they are no longer last.

§ 5.2
Assert the fourth generated button click clears the chain, leaves exact text finished, and exposes a computed or inspectable white border on .completion-message.

## Acceptance criteria

§ 6.1
The automated command exits zero only when all button-count, clear-document, exact-text, and white-border assertions pass.

§ 6.2
The command output is saved by the future executor as verification evidence without mutating implementation files.

## Handoff

§ 7.1
W05 can proceed only after the deterministic DOM verification passes.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
