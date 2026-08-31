# Step: 09-step-mode-surface

## Ownership

- Goal: `02-page-shell`
- Work unit: `W51`
- Type: `source`

## Change target

- File: `src/plan-overview/src/render/shell.rs`
- Primary symbol or file scope: `render_mode_surface()`
- Subscope: `N/A`

## Objective

§ 4.1
Let the mode decide what leads, without removing anything the other modes need.

## Instructions

§ 5.1
State the mode on the page and select the leading surface: soundness in planning, execution in implementing, outcome in complete. A surface not leading in the current mode remains reachable through navigation and is never omitted. Zero progress in planning mode is presented as not started, never as failure.

## Acceptance criteria

§ 6.1
In each mode the stated mode matches the derivation, the leading surface differs, and every non-leading surface is still reachable. A planning-mode plan shows no next action that execution has not reached. US-61, US-62, US-63 and US-66 apply.

## Handoff

§ 7.1
W67 animates the transition when the mode changes under a live update; W63 verifies the crossing.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
