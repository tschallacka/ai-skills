# Step: 03-step-state-stream

## Ownership

- Goal: `10-live-updates`
- Work unit: `W60`
- Type: `source`

## Change target

- File: `src/plan-overview/src/serve.rs`
- Primary symbol or file scope: `state_stream()`
- Subscope: `N/A`

## Objective

§ 4.1
Deliver change to an open page without it polling the whole artifact.

## Instructions

§ 5.1
Publish state changes to connected pages as a stream over the same server. A client connecting or reconnecting first receives the current state, then subsequent changes, so a page that was closed during a change is correct on reopen rather than replaying history.

## Acceptance criteria

§ 6.1
An open page receives changes without polling the artifact; a page reopened after a missed change shows the current state; two connected pages each receive the stream independently. US-32, US-49 and US-72 apply.

## Handoff

§ 7.1
W61 consumes the stream client-side; W50 mode changes travel over it, which US-66 exercises.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
