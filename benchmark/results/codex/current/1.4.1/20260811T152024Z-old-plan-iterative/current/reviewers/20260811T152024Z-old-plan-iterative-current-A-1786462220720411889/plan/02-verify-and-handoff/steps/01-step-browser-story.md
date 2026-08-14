# Step: 01-step-browser-story

## Ownership

- Goal: `02-verify-and-handoff`
- Work unit: `W05`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `Browser story US-01`
- Subscope: `N/A`

## Objective

§ 4.1
After implementation, click through the button chain and verify the fourth generated button clears the document and shows finished with a visible white border.

## Instructions

§ 5.1
Open the future completed button-chain.html in a browser and execute US-01 using real mouse clicks on the current last visible button: initial button, generated button 1, generated button 2, generated button 3, then generated button 4.

## Acceptance criteria

§ 6.1
Pass only if each non-final click appends exactly one button below the previous last button, the final generated-button-4 click clears the document, and the only completion content is exact lowercase finished with a visible white border.

## Handoff

§ 7.1
The final handoff must include the browser evidence, story status, and bug-register status. If US-01 fails, record a bug and do not mark the initiative complete.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
