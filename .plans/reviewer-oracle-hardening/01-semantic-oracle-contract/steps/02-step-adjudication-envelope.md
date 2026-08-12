# Goal 01 / Step 02: Define the redacted adjudication envelope

## Ownership

- Goal: `01-semantic-oracle-contract`
- Work unit: `W03`
- Type: `source`

## Change target

- File: `benchmark/planning/review-oracle.sh`
- Primary symbol or file scope: `redacted adjudication envelope`
- Subscope: `candidate finding envelope`

## Objective

Define the independent oracle input/output contract without exposing hidden seed
IDs or private manifest material to Reviewer B.

## Instructions

Specify the candidate envelope fields as required strings
(`finding_id`, `path`, `location`, `summary`, `evidence`,
`required_correction`) plus required boolean `independent`. Specify private
rows as required string `defect_id`, string array `finding_ids`, enum
`classification`, enum `confidence`, and string `rationale`. Define
true-positive, unresolved, ambiguous, duplicate, and false-positive transitions
using the plan-level normalization and precedence rules. Redact keys, private
roots, mutation strings, and hidden IDs from reviewer and published artifacts.

## Acceptance criteria

- The envelope supports one finding covering several defects.
- Missing path/location/correction evidence becomes unresolved.
- The independent oracle is the only role allowed to read private manifest data.

## Handoff

Goal 02 consumes the stable oracle envelope and state fields.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
