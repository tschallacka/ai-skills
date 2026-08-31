# Step: 08-step-derive-mode

## Ownership

- Goal: `02-page-shell`
- Work unit: `W50`
- Type: `source`

## Change target

- File: `src/plan-overview/src/plan/mode.rs`
- Primary symbol or file scope: `derive_mode()`
- Subscope: `N/A`

## Objective

§ 4.1
Decide which lifecycle the plan is in, because planning and implementing ask different questions of the same data.

## Instructions

§ 5.1
Derive planning while the review is pending or no step has started, implementing once the review is approved and at least one step has started, and complete when every step and its applicable verification have passed. A combination satisfying none of these, such as an approved plan with no steps, is reported as ambiguous with the contradiction named.

## Acceptance criteria

§ 6.1
Each of the three modes is derived from its defining condition, and a contradictory plan reports ambiguous naming what conflicted rather than defaulting to a mode. US-61 to US-63 and US-65 depend on this.

## Handoff

§ 7.1
W51 selects the leading surface from the mode; W53 selects what autoplay follows; neither re-derives it.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
