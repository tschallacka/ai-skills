# Goal 02 / Step 06: Align the benchmark worker prompt

## Ownership

- Goal: `02-approval-state-integrity`
- Work unit: `W15`
- Type: `docs`

## Change target

- File: `benchmark/planning/worker-prompt.md`
- Primary symbol or file scope: `worker reviewer prompt`
- Subscope: `protocol evidence`

## Objective

Make benchmark workers produce evidence compatible with semantic adjudication and
the approval-state contract.

## Instructions

Require precise path/location, contradiction, correction, independent ownership,
and boolean overall approval evidence. State that consolidated AR findings are
valid and historical versions are never rerun.

## Acceptance criteria

The prompt requires all fields needed by W03/W04 and does not expose private
seed IDs or manifest material.

## Handoff

W11 uses the aligned prompt in the current-only gate.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
