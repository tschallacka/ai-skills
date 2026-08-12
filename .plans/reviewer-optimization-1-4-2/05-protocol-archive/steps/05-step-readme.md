# Step: 05-step-readme

## Ownership

- Goal: `05-protocol-archive`
- Work unit: `W25`
- Type: `docs`

## Change target

- File: `benchmark/planning/README.md`
- Primary symbol or file scope: `benchmark operator workflow`
- Subscope: `N/A`

## Objective

§ 4.1
Document the user-facing setup command, hidden protocol metadata, capsule lifecycle, pilot command, and result archive locations without exposing implementation differences.

## Instructions

§ 5.1
Work only on `benchmark/planning/README.md`, targeting `benchmark operator workflow`. Document the user-facing setup command, hidden protocol metadata, capsule lifecycle, pilot command, and result archive locations without exposing implementation differences. Keep unrelated files and symbols out of this step; preserve prerequisite contracts and evidence boundaries.

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
