# Step: 06-step-verify-frame-budget

## Ownership

- Goal: `11-visual-language`
- Work unit: `W69`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `frame budget during graph growth`
- Subscope: `N/A`

## Objective

§ 4.1
Measure the animation rather than asserting it is smooth.

## Instructions

§ 5.1
Animate graph growth on the 337 KB fixture and record the frame times and the cinematic tier the page settled at. Record numbers, not impressions.

## Acceptance criteria

§ 6.1
Frame times and the settled tier are recorded. A budget miss at the full tier on a slow machine is evidence the degradation works; a miss that does not step the tier down is a finding. US-76 and US-80 apply.

## Handoff

§ 7.1
These numbers justify the tier thresholds W71 declares.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
