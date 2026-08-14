# Step: 01-step-browser-story-us-01

## Ownership

- Goal: `02-ui-story-verification`
- Work unit: `W05`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `Browser story US-01`
- Subscope: `N/A`

## Objective

§ 4.1
Future browser verification clicks the initial, first generated, second generated, third generated, and fourth generated buttons, confirming one appended button per last-button click and the terminal finished state with visible white border.

## Instructions

§ 5.1
In the future execution run, open button-chain.html in a browser and use only direct user-facing clicks on the visible current last button.

§ 5.2
Perform five clicks: initial button, generated 1, generated 2, generated 3, then generated 4 as the terminal click. Record the observed button count after each append-producing click and the terminal finished message after the final click.

## Acceptance criteria

§ 6.1
US-01 passes only when clicks 1-4 each append exactly one button below the previous last button and click 5 on generated 4 clears the document.

§ 6.2
The final visible content includes the exact lowercase text finished with a visible white border, and no unresolved bugs remain in bugs.md.

## Handoff

§ 7.1
Record future evidence in ui-user-stories.md and ui-story-runs/US-01.md; if the story fails, add bug feedback-loop goals before claiming completion.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
