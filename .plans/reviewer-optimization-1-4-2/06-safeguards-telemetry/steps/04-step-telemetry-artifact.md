# Step: 04-step-telemetry-artifact

## Ownership

- Goal: `06-safeguards-telemetry`
- Work unit: `W31`
- Type: `source`

## Change target

- File: `benchmark/planning/telemetry.sh`
- Primary symbol or file scope: `ROLLOUT_FILE output path`
- Subscope: `N/A`

## Objective

§ 4.1
Emit validated machine-readable per-worker/reviewer telemetry with exact-versus-heuristic fields, provenance, retention paths, and lifecycle records.

## Instructions

§ 5.1
Define the retained raw telemetry artifact path and handoff contract only: the artifact must carry protocol/version/schema identifiers and point to lifecycle and provenance records; extraction and validation belong to W53/W60.

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
