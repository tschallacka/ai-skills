# Step: 03-step-append-behavior

## Ownership

- Goal: `01-build-button-chain`
- Work unit: `W03`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `appendNextButton()`
- Subscope: `N/A`

## Objective

§ 4.1
Append exactly one generated button below the current last button and move last-button authority to the new button.

## Instructions

§ 5.1
Implement appendNextButton() so clicking the current last button appends exactly one new generated button below it.

§ 5.2
After appending, make the new generated button the current last button. Older buttons must not append additional buttons once they are no longer the last button.

## Acceptance criteria

§ 6.1
Source review shows a single append path per valid click, generatedCount increments by one per append, and stale buttons cannot continue appending after a newer last button exists.

## Handoff

§ 7.1
W04 can rely on generatedCount or equivalent state to identify when the fourth generated button has been reached.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
