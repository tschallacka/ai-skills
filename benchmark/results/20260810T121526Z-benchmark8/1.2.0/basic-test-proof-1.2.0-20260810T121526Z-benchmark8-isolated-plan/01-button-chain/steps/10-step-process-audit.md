# Step 10: process cleanup audit

## Ownership
- Goal: `01-button-chain`
- Work unit: `W10`
- Type: `verification`

## Change target
- File: `N/A`
- Primary symbol or file scope: `worker process-group cleanup audit`
- Subscope: `N/A`

## Objective
Confirm no browser, server, or driver process started by this worker remains after the proof.

## Instructions
1. Inspect the worker-owned process group using the runner's process audit method, matching only browser/server/driver processes descended from this worker and excluding unrelated host or parallel-worker processes.

## Acceptance criteria
- The matching process list is empty; this proof itself starts no browser, server, driver, or other execution tooling.

## Handoff
- W08 records the zero-result audit and its scope in the final analysis report.

## Atomicity check
- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
