# Step: 06-step-verify-live

## Ownership

- Goal: `10-live-updates`
- Work unit: `W63`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `edit on disk, page follows`
- Subscope: `N/A`

## Objective

§ 4.1
Prove the chain from a helper writing a file to the page moving.

## Instructions

§ 5.1
With the binary serving, edit a plan document through a planning helper. Confirm the page follows within the stated interval, the graph animated rather than redrew, and scroll, expansion and tab selection survived. Then run a helper that writes three files and confirm one visible update. Then close the page, change the plan, reopen and confirm current state.

## Acceptance criteria

§ 6.1
US-32, US-70, US-71 and US-72 pass with recorded interactions and the observed latency recorded against the stated interval.

## Handoff

§ 7.1
Completes the live half of the tool; the animation quality is measured separately by W69.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
