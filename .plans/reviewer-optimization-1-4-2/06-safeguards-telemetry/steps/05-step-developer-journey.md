# Step: 05-step-developer-journey

## Ownership

- Goal: `06-safeguards-telemetry`
- Work unit: `W32`
- Type: `docs`

## Change target

- File: `benchmark/planning/analyzer-prompt.md`
- Primary symbol or file scope: `developer journey evidence rules`
- Subscope: `N/A`

## Objective

§ 4.1
Require concise per-version journeys covering review rounds, findings, fixes, validation attempts, artifact expansion, and latency/token deltas; label missing evidence unavailable.

## Instructions

§ 5.1
Work only on `benchmark/planning/analyzer-prompt.md`, targeting `developer journey evidence rules`. Require concise per-version journeys covering review rounds, findings, fixes, validation attempts, artifact expansion, and latency/token deltas; label missing evidence unavailable. Keep unrelated files and symbols out of this step; preserve prerequisite contracts and evidence boundaries.

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
