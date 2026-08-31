# Step: 01-step-watch-plan-dir

## Ownership

- Goal: `10-live-updates`
- Work unit: `W58`
- Type: `source`

## Change target

- File: `src/plan-overview/src/watch.rs`
- Primary symbol or file scope: `watch_plan_dir()`
- Subscope: `N/A`

## Objective

§ 4.1
Notice that the plan changed, which is what makes every live behaviour possible.

## Instructions

§ 5.1
Detect changes under the plan directory and report them as change events. Dependency-free means no filesystem-notification crate, so use a bounded scan of path, modification time and size across the plan tree at a stated interval. State the interval and the tree size rather than tuning them invisibly. The mechanism must behave identically on Linux, macOS and Windows.

## Acceptance criteria

§ 6.1
A change to any plan document produces an event within the stated interval; a file outside the plan directory produces none; a file touched without content change produces none. The interval and the observed detection latency are recorded.

## Handoff

§ 7.1
W59 coalesces these events; W60 publishes them. The stated interval is the floor on how fresh the page can be.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
