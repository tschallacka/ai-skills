# Step: 05-step-validate-approval-identity

## Ownership

- Goal: `02-authority-and-provenance`
- Work unit: `W13`
- Type: `source`

## Change target

- File: `benchmark/planning/setup-benchmark.sh`
- Primary symbol or file scope: `approval_schema_validator()`
- Subscope: `N/A`

## Objective

§ 4.1
Validate the approval.json schema, required approval metadata, and complete finding object types before serialization. Session/capsule binding is owned by W15.

## Instructions

§ 5.1
Define the required approval envelope: reviewer identity/session metadata, mode, approval timestamp, overall decision, approved findings, rejected findings, and complete machine-readable finding objects. Reject missing fields, wrong types, empty required values, duplicate finding IDs, and ID-only or narrative-only findings with stable reason codes. Do not perform lifecycle-session binding here.

## Acceptance criteria

§ 6.1
Malformed approval input produces a structured schema-invalid result with per-field/per-finding reason codes and cannot be serialized as terminal oracle evidence; a valid envelope reaches W15 with all required fields intact.

## Handoff

§ 7.1
W15 receives a schema-valid approval object whose required metadata and finding field types have passed validation.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
