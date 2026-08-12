# Step: 02-step-reviewer-generator

## Ownership

- Goal: `01-contract-profile`
- Work unit: `W02`
- Type: `source`

## Change target

- File: `planning/scripts/generate-reviewer.sh`
- Primary symbol or file scope: `main profile-generation flow`
- Subscope: `N/A`

## Objective

§ 4.1
Enforce REVIEWER_SECTION allowlisting, required-section validation, source hashing, version 1.4.2 metadata, and non-hand-edited output generation.

## Instructions

§ 5.1
Work only on `planning/scripts/generate-reviewer.sh`, targeting `main profile-generation flow`. Enforce REVIEWER_SECTION allowlisting, required-section validation, source hashing, version 1.4.2 metadata, and non-hand-edited output generation. Keep unrelated files and symbols out of this step; preserve prerequisite contracts and evidence boundaries.

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
