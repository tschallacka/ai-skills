# Goal 01 / Step 03: Grade semantic consolidated findings

## Ownership

- Goal: `01-semantic-oracle-contract`
- Work unit: `W02`
- Type: `source`

## Change target

- File: `benchmark/planning/grade-blinded-run.sh`
- Primary symbol or file scope: `semantic adjudication`
- Subscope: `one-to-many finding coverage`

## Objective

Replace exact hidden-ID matching as the authoritative score with independent
semantic adjudication that can map one reviewer finding to several defects.

## Files or areas

`benchmark/planning/grade-blinded-run.sh` only. W03 owns envelope production;
W02 consumes its contract and owns semantic grading/report calculations.

## Instructions

Consume the W03 envelope and apply the plan-level normalization, matching,
precedence, and classification rules. Report semantic true positives, false
negatives, unresolved matches, false positives, independent-catch rate, and
mechanical exact-ID rate separately. Do not modify setup, prompt, or analyzer
surfaces owned by later work units.

## Acceptance criteria

- One AR finding covering SD-01/02/03 yields three semantic true positives.
- A finding without sufficient location/correction evidence is unresolved.
- Reports contain no defect paths or secrets beyond the permitted redacted
  evidence.
- Oracle role enforcement and transcript integrity remain fail-closed.

## Handoff

Goal 02 consumes the semantic report fields and adds approval-state handling.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
