# Goal 02 / Step 07: Publish protocol metadata and evaluation state

## Ownership

- Goal: `02-approval-state-integrity`
- Work unit: `W16`
- Type: `source`

## Change target

- File: `benchmark/planning/setup-benchmark.sh`
- Primary symbol or file scope: `protocol metadata`
- Subscope: `evaluation state`

## Objective

Publish the explicit state schema and fail-closed reasons without inferring
adoption from worker exit or oracle completion alone.

## Instructions

Consume W04's internal approval-state result and emit the booleans, rates,
denominator, reason enums, conflict flag, protocol metadata, and evaluation
fields defined in the plan. This step exclusively owns publication; W04 owns
only the internal approval-state computation.

## Acceptance criteria

Evaluation and protocol metadata agree on every state field and preserve false
approval as non-adoptable evidence.

## Handoff

W13 tests this schema; W11 verifies it in the published current archive.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
