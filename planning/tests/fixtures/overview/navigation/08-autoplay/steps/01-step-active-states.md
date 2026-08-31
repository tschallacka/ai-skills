# Step: 01-step-active-states

## Ownership

- Goal: `08-autoplay`
- Work unit: `W41`
- Type: `source`

## Change target

- File: `src/plan-overview/src/pages/autoplay.rs`
- Primary symbol or file scope: `active_states()`
- Subscope: `N/A`

## Objective

§ 4.1
Derive what is currently active from the served state, so autoplay follows reality rather than a tracker.

## Instructions

§ 5.1
Compute the set of active states from the served state: steps marked in progress and the goals holding them. Do not read the progress documents, which record what a tracker was last told. An empty set is a valid result meaning nothing is active.

## Acceptance criteria

§ 6.1
One in-progress step yields one active state, several yield one each, none yields an empty set, and the derivation ignores the progress documents entirely. US-28 depends on the empty case being explicit.

## Handoff

§ 7.1
W42 follows this set; W43 renders one tab per member; W44 pins the derivation.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
