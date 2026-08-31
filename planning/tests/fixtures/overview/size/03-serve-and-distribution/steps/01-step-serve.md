# Step: 01-step-serve

## Ownership

- Goal: `03-serve-and-distribution`
- Work unit: `W14`
- Type: `source`

## Change target

- File: `src/plan-overview/src/serve.rs`
- Primary symbol or file scope: `serve()`
- Subscope: `N/A`

## Objective

§ 4.1
Serve the artifact and the live state from the binary, so one implementation replaces four that could disagree.

## Instructions

§ 5.1
Bind the loopback interface, print the bound port on a line the invoker can read immediately, and serve the artifact plus the state. Resolve the host name literally rather than by reverse lookup, which is what stalled a previous rung on macOS. No response is produced by slicing rendered HTML with a pattern.

## Acceptance criteria

§ 6.1
The port is printed before the first request is possible, the artifact and the state are both served, and stopping the process leaves nothing listening. US-32, US-49 and US-50 apply, including the page stating that the server is gone rather than rendering blank.

## Handoff

§ 7.1
W60 adds the change stream to this server; W58 supplies its events. W15 and W16 may only be removed once this serves.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
