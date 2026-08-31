# Step: 02-step-coalesce-events

## Ownership

- Goal: `10-live-updates`
- Work unit: `W59`
- Type: `source`

## Change target

- File: `src/plan-overview/src/watch.rs`
- Primary symbol or file scope: `coalesce_events()`
- Subscope: `N/A`

## Objective

§ 4.1
Turn a helper writing several files into one update rather than several redraws.

## Instructions

§ 5.1
Collapse events arriving within a stated debounce window into one change event. State the window rather than tuning it silently, and ensure a long sequence of edits still produces updates rather than being deferred indefinitely.

## Acceptance criteria

§ 6.1
A single edit yields one event; a burst within the window yields one; a continuous stream of edits still yields periodic events rather than none. US-71 requires the observed behaviour to match the stated window.

## Handoff

§ 7.1
W62 pins these three cases; W56 depends on receiving one before-and-after pair rather than several.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
