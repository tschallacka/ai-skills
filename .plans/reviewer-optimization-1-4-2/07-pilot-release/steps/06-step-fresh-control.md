# Step: 06-step-fresh-control

## Ownership

- Goal: `07-pilot-release`
- Work unit: `W55`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `matched fresh-review control run`
- Subscope: `N/A`

## Objective

§ 4.1
Run the same one-to-two revision matrix with default fresh-review mode, matching task, environment, artifact checks, and telemetry requirements.

## Instructions

§ 5.1
Use an existing fresh-review archive as immutable control evidence when its task, revision, metadata, and evidence are compatible. Run a new fresh-review control only when the current 1.4.2 comparison lacks a compatible control; never rerun an older version solely to retrofit modern protocol fields.

## Acceptance criteria

§ 6.1
Exactly one accepted fresh-review archive exists for each selected revision, each has complete telemetry and final validation, and its run metadata matches the iterative task/revision matrix while retaining a distinct run ID and session set.

## Handoff

§ 7.1
Record the changed target, command/output evidence, and unavailable or environment-specific results for the next dependency.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
