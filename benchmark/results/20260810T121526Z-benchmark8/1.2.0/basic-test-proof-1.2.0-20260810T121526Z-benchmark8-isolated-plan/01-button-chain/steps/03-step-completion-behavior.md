# Step 03: completion behavior

## Ownership
- Goal: `01-button-chain`
- Work unit: `W03`
- Type: `source`

## Change target
- File: `button-chain.html`
- Primary symbol or file scope: `clearAndShowFinished()`
- Subscope: `fourth generated-button branch`

## Objective
Clear the document on the fourth generated-button press and insert only the exact lowercase completion text.

## Instructions
1. Implement the terminal branch so generated `Button 4`'s mouse click, after `Button 0` through `Button 3` have created generated buttons 1–4, removes the prior document content and renders `finished` exactly once.

## Acceptance criteria
- The `Button 4` terminal click leaves no chain buttons and the visible text is exactly `finished`; no fifth button is appended.

## Handoff
- W04 can style the single completion message without changing its text or behavior.

## Atomicity check
- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
