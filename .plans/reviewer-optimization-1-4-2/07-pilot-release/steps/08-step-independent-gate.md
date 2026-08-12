# Step: 08-step-independent-gate

## Ownership

- Goal: `07-pilot-release`
- Work unit: `W57`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `final independent review gate`
- Subscope: `N/A`

## Objective

§ 4.1
Require a fresh final reviewer approval, no open AR findings, complete lifecycle handoff records, and preserved Reviewer B session evidence before release validation passes.

## Instructions

§ 5.1
Work only on `N/A`, targeting `final independent review gate`. Require a fresh final reviewer approval, no open AR findings, complete lifecycle handoff records, and preserved Reviewer B session evidence before release validation passes. Keep unrelated files and symbols out of this step; preserve prerequisite contracts and evidence boundaries.

## Acceptance criteria

§ 6.1
W57 writes final-independent-gate.json with reviewer B session/capsule IDs, approval.json hash, open AR finding count, lifecycle handoff hash, telemetry status, and archive paths; W38 consumes this object and fails if any field is missing or false.

## Handoff

§ 7.1
Record the changed target, command/output evidence, and unavailable or environment-specific results for the next dependency.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
