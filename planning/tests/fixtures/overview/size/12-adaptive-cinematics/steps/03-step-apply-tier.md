# Step: 03-step-apply-tier

## Ownership

- Goal: `12-adaptive-cinematics`
- Work unit: `W73`
- Type: `source`

## Change target

- File: `src/plan-overview/assets/perf.js`
- Primary symbol or file scope: `applyTier`
- Subscope: `N/A`

## Objective

§ 4.1
Switch the whole style layer at once, and tell the reader why it looks plain.

## Instructions

§ 5.1
Apply the selected tier as one attribute on the root so the style layer switches wholesale, and show the current tier where a reader can find it, distinguishing a measured step-down from the reduced-motion preference. The reduced-motion preference pins the minimal tier and no measurement raises it.

## Acceptance criteria

§ 6.1
One attribute governs the tier, the indicator states the tier and its cause, the reduced-motion preference pins minimal irrespective of measured speed, and no tier hides a value, link or warning. US-80 to US-84 apply.

## Handoff

§ 7.1
W82 style rules key off this attribute; W74 pins the transitions between tiers.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
