# Step: 05-step-browser-verification

## Ownership

- Goal: `01-button-chain`
- Work unit: `W05`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `US-01 browser flow`
- Subscope: `N/A`

## Objective

§ 4.1
Future browser verification: perform five real user clicks: initial button to create generated button one, generated button one to create generated button two, generated button two to create generated button three, generated button three to create generated button four, then generated button four to clear the document and confirm exact finished text with a visible white border.

## Instructions

§ 5.1
In future execution only, open button-chain.html in a browser and perform real clicks on the visible current last button. Click the initial button to create generated button one, click generated button one to create generated button two, click generated button two to create generated button three, and click generated button three to create generated button four if needed by the implementation flow; then press the fourth generated button to trigger completion.

§ 5.2
Record observable evidence after each click: button count and ordering for append states, then final document clearing, exact finished text, and visible white border. Do not use console commands, injected events, or DOM mutation as passing evidence.

## Acceptance criteria

§ 6.1
US-01 is marked passed only after real browser clicks prove the first four clicks each append exactly one generated button below the previous last button, and the fifth click on the fourth generated button completes instead of appending a fifth generated button.

§ 6.2
Final evidence shows the previous document content cleared and exact lowercase finished displayed with a visible white border.

## Handoff

§ 7.1
Handoff for completion: record browser route, viewport, click sequence, observed states, final screenshot or equivalent visual evidence, and final pass/fail status in the UI story run cache.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
