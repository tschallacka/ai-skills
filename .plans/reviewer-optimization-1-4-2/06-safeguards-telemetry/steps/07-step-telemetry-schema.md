# Step: 07-step-telemetry-schema

## Ownership

- Goal: `06-safeguards-telemetry`
- Work unit: `W52`
- Type: `config`

## Change target

- File: `benchmark/planning/telemetry-schema.json`
- Primary symbol or file scope: `raw telemetry schema`
- Subscope: `N/A`

## Objective

§ 4.1
Define the validated machine-readable schema for worker/reviewer IDs, provenance, phase boundaries, token/cache composition, counts, durations, tool volumes, validator/patch/function-call metrics, command activity, and parent/child lifecycle.

## Instructions

§ 5.1
Define field-level JSON schema for identity/provenance/status, phase_records[{phase,start,end,duration_seconds,source,precision}], reviewer_records[{session_id,cycle,verification_pass,events,token_total,source}], metrics[{name,value,status in {exact,heuristic,unavailable},reason}], and taint_causes[]. Conditional reviewer_session_id is required when reviewer_mode is iterative or fresh-review; telemetry_source and database/rollout provenance are always required.

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
