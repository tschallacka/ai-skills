# Step: 03-step-pilot-run

## Ownership

- Goal: `07-pilot-release`
- Work unit: `W36`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `one-to-two revision pilot command`
- Subscope: `N/A`

## Objective

§ 4.1
Run the bounded 1.4.2 pilot against one or two revisions with iterative mode and mandatory fresh final review, retaining complete archives and telemetry.

## Instructions

§ 5.1
Use existing archived reports as-is when their metadata and evidence are compatible. If a current 1.4.2 run is required, run only the current protocol against the selected task/revision; do not rerun or patch historical v1.3.1/v1.4.1 reports to make them satisfy modern requirements.

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
