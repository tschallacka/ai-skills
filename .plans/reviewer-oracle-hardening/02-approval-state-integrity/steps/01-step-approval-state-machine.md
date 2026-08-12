# Goal 02 / Step 01: Enforce approval-state semantics

## Ownership

- Goal: `02-approval-state-integrity`
- Work unit: `W04`
- Type: `source`

## Change target

- File: `benchmark/planning/setup-benchmark.sh`
- Primary symbol or file scope: `approval state`
- Subscope: `review/adoption state machine`

## Objective

Make the harness distinguish terminal review evidence from successful plan
approval and expose an internal state result for the dedicated publication
step. W16 publishes protocol metadata and evaluation.

## Files or areas

`benchmark/planning/setup-benchmark.sh` approval-state branch only. W05/W14/W15
own contract and prompt surfaces; W16 owns metadata/evaluation publication.

## Instructions

Keep Reviewer B false approval available for oracle grading. Compute adoption
eligibility false when approval is false, missing, malformed, or conflicting.
Use explicit machine-readable internal state rather than inferring status from
exit codes or oracle completion. Do not publish protocol metadata/evaluation or
change reviewer projection, worker prompt, or analyzer prompt in this step.

## Acceptance criteria

- Valid false approval reaches the oracle but produces non-adoptable status.
- Missing or conflicting approval fails closed with a specific reason.
- Valid true approval can become adoptable only after all other thresholds pass.

## Handoff

Goal 03 receives explicit machine-readable state fields and fail-closed reasons.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
