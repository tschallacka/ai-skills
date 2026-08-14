# Step: 02-step-browser-story

## Ownership

- Goal: `02-verify-button-chain-behavior`
- Work unit: `W05`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `US-01 browser story`
- Subscope: `N/A`

## Objective

§ 4.1
Run the direct browser story that clicks the current last button five times, confirms one button is appended when clicking the initial button and generated buttons 1 through 3, and confirms clicking generated button 4 clears the document and shows finished with a visible white border.

## Instructions

§ 5.1
Open the implemented button-chain.html in a browser from a fresh state. Run ui-story-runs/US-01.md exactly: click the rendered current last button five times, choosing the bottom-most visible button after each append.

§ 5.2
Record visible evidence after each click. Use only normal browser input to create the passing state.

§ 5.3
After clicking generated button 4, confirm only finished remains visible and that the finished text has a visible white border. Update ui-user-stories.md, ui-story-runs/US-01.md, and bugs.md with the result.

## Acceptance criteria

§ 6.1
US-01 is marked passed only after browser evidence shows one button added by clicking the initial button and generated buttons 1 through 3, followed by generated button 4 producing the terminal state.

§ 6.2
No unresolved bug remains in bugs.md and no UI story remains untested, in progress, or bug found.

§ 6.3
If browser evidence fails, add investigation and fix goals before declaring the plan executable work complete.

## Handoff

§ 7.1
The final handoff records the browser evidence location, story status, and any resolved bug IDs.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
