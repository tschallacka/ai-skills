# Step 02: append behavior

## Ownership
- Goal: `01-button-chain`
- Work unit: `W02`
- Type: `source`

## Change target
- File: `button-chain.html`
- Primary symbol or file scope: `appendNextButton()`
- Subscope: `click handler callback`

## Objective
Make each click on the current last button append exactly one button below it and make that new button the current last target.

## Instructions
1. Implement `appendNextButton()` in the future HTML file, preserving a single explicit handler per current last button, one append operation per click, and sequential accessible labels `Button 1` through `Button 4`.

## Acceptance criteria
- Three successive clicks produce one new button per click, in document order, with no duplicate append.

## Handoff
- W03 receives a deterministic generated-button count and terminal click target.

## Atomicity check
- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
