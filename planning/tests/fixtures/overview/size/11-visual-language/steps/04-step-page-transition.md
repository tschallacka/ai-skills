# Step: 04-step-page-transition

## Ownership

- Goal: `11-visual-language`
- Work unit: `W67`
- Type: `source`

## Change target

- File: `src/plan-overview/src/render/shell.rs`
- Primary symbol or file scope: `render_transition()`
- Subscope: `N/A`

## Objective

§ 4.1
Make a route change read as movement, and make direction meaningful.

## Instructions

§ 5.1
Emit the markup and classes that let a route change animate, including the direction of travel so moving deeper and moving back differ visibly. A transition never delays the reader from acting on the arrived page.

## Acceptance criteria

§ 6.1
Forward and back are visibly different, the arrived page is interactive immediately, and the transition respects the motion tokens and the current tier. US-74 applies.

## Handoff

§ 7.1
W51 uses the same mechanism when the mode changes under a live update.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
