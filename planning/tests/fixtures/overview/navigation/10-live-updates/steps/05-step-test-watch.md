# Step: 05-step-test-watch

## Ownership

- Goal: `10-live-updates`
- Work unit: `W62`
- Type: `test`

## Change target

- File: `src/plan-overview/tests/watch.rs`
- Primary symbol or file scope: `coalescing_and_scan`
- Subscope: `N/A`

## Objective

§ 4.1
Pin the watcher so liveness cannot become either noisy or silent.

## Instructions

§ 5.1
Assert one edit yields one event, a burst within the debounce yields one, a change outside the plan directory yields none, and a touch with no content change yields none. Fault-inject by shortening the debounce to zero and requiring the burst case to fail.

## Acceptance criteria

§ 6.1
All four cases pinned, and the zero-debounce injection makes the burst case fail rather than passing by luck.

## Handoff

§ 7.1
W62 guards W58 and W59; W63 verifies the whole chain in the browser.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
