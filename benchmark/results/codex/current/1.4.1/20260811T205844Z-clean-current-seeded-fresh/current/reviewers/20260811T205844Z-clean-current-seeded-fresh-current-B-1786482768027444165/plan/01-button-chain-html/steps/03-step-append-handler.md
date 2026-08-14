# Step: 03-step-append-handler

## Ownership

- Goal: `01-button-chain-html`
- Work unit: `W03`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `appendNextButton()`
- Subscope: `N/A`

## Objective

§ 4.1
Implement the click handler so only the current last button appends exactly one new button below it.

## Instructions

§ 5.1
Future executor implements appendNextButton() so a click on the current last button appends exactly one new button after it, updates current-last ownership to the new button, and prevents earlier buttons from appending additional buttons. Generated-button numbering must count generated buttons only.

## Acceptance criteria

§ 6.1
From load, click 1 on the initial button creates generated button 1; click 2 on generated 1 creates generated 2; click 3 on generated 2 creates generated 3; click 4 on generated 3 creates generated 4. Each click increases the button count by exactly one and older non-last buttons do not append.

## Handoff

§ 7.1
W04 can rely on generated button 4 existing only after the fourth append-producing click and being the current last button.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
