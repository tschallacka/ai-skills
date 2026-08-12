# Goal 02 / Step 03: Define analyzer and report state schema

## Ownership

- Goal: `02-approval-state-integrity`
- Work unit: `W06`
- Type: `docs`

## Change target

- File: `benchmark/planning/analyzer-prompt.md`
- Primary symbol or file scope: `report schema`
- Subscope: `approval metrics`

## Objective

Make analyzer output preserve the approval/adoption truth table and semantic
oracle metrics without inferring a pass from worker exit code.

## Instructions

Require booleans `review_completed`, `plan_approved`, `oracle_completed`, and
`adoptable`; arrays `fail_closed_reasons`; semantic and mechanical rates; and
explicit denominators. Define reasons for missing, conflicting, tainted, below-
threshold, ambiguous, and false-approval states. Require current-protocol
archives and forbid historical report rewriting.

## Acceptance criteria

- Analyzer instructions define the exact adoption predicate.
- False approval is reported as gradeable but non-adoptable.
- Missing denominators and conflicting evidence remain fail-closed.

## Handoff

Goal 03 tests the report fields and truth table end to end.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
