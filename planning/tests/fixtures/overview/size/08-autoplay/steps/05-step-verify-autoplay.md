# Step: 05-step-verify-autoplay

## Ownership

- Goal: `08-autoplay`
- Work unit: `W45`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `autoplay follows a changing state`
- Subscope: `N/A`

## Objective

§ 4.1
Prove autoplay follows real change and respects a browsing reader.

## Instructions

§ 5.1
With the binary serving, toggle autoplay on, change the served state so a different step becomes active, and confirm the page follows. Open a second active state, toggle between tabs, end one and confirm its tab vanishes, then end the last and confirm the explicit no-active-step state. Repeat one change with autoplay off and confirm the page does not move.

## Acceptance criteria

§ 6.1
US-09, US-10, US-26, US-27 and US-28 pass with recorded interactions, including the off case not navigating.

## Handoff

§ 7.1
Evidence for the monitor half of the tool; the reviewing half is covered by the page goals.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
