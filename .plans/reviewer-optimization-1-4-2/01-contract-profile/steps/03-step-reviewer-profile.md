# Step: 03-step-reviewer-profile

## Ownership

- Goal: `01-contract-profile`
- Work unit: `W03`
- Type: `generated`

## Change target

- File: `planning/REVIEWER.md`
- Primary symbol or file scope: `generated reviewer profile`
- Subscope: `N/A`

## Objective

§ 4.1
Regenerate the reviewer profile from the tagged source sections and record the source hash and version without hand edits.

## Instructions

§ 5.1
Work only on `planning/REVIEWER.md`, targeting `generated reviewer profile`. Regenerate the reviewer profile from the tagged source sections and record the source hash and version without hand edits. Keep unrelated files and symbols out of this step; preserve prerequisite contracts and evidence boundaries.

## Acceptance criteria

§ 6.1
The named target has the required behavior, its output is bounded and reproducible, and the companion or downstream verification can observe the result without an unnamed change.

## Handoff

§ 7.1
Record the changed target, command/output evidence, and unavailable or environment-specific results for the next dependency.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
