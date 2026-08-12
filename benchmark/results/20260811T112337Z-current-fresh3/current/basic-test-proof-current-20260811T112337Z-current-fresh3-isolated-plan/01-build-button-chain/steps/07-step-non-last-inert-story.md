# Step: 07-step-non-last-inert-story

## Ownership

- Goal: `01-build-button-chain`
- Work unit: `W08`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `US-02 non-last inert browser flow`
- Subscope: `N/A`

## Objective

§ 4.1
Using direct clicks, verify that clicking an earlier non-last button after generated buttons exist does not append a button, clear the document, or trigger finished.

## Instructions

§ 5.1
After future implementation, run US-02 through the browser using direct mouse clicks only. Create at least two generated buttons through valid current-last clicks, then click an earlier button that is no longer last. Do not use console, injected JavaScript, storage edits, or direct API calls.

## Acceptance criteria

§ 6.1
US-02 passes only when the non-last click leaves the button count unchanged, does not clear the document, and does not show finished. Record route, click sequence, before/after count, and observed result in ui-story-runs/US-02.md.

## Handoff

§ 7.1
W07 can cite US-02 evidence for the last-button guard contract during the static audit.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
