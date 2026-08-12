# Goal 02 / Step 04: Test analyzer state schema

## Ownership

- Goal: `02-approval-state-integrity`
- Work unit: `W13`
- Type: `test`

## Change target

- File: `benchmark/planning/tests/test-review-lifecycle.sh`
- Primary symbol or file scope: `analyzer state schema test`
- Subscope: `report truth table`

## Objective

Prevent analyzer output from collapsing review completion, approval, oracle
completion, and adoption into one status.

## Instructions

Exercise true, false, missing, conflicting, tainted, below-threshold, and
ambiguous inputs. Assert required booleans, reason enums, denominators, and the
exact adoptable predicate.

## Acceptance criteria

Every invalid or incomplete state is explicitly non-adoptable and reports its
fail-closed reason.

## Handoff

W11 uses these assertions in the full current-protocol verification.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
