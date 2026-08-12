# Step: 03-step-integrity-monitor-tests

## Ownership

- Goal: `10-plan-integrity-and-execution-persistence`
- Work unit: `W66`
- Type: `test`

## Change target

- File: `planning/tests/test-plan-integrity-and-monitor.sh`
- Primary symbol or file scope: `integrity monitor fixture tests`
- Subscope: `N/A`

## Objective

Verify that plan additions are complete and that monitoring continues through intermediate subprocess states.

## Instructions

Add focused shell fixtures for note-only and complete plan mutations, status-only worker output followed by progress, true failure, retry-budget exhaustion, accepted completion, and interruption cleanup. Assert that validator/review requirements, bounded steering, terminal classification, and evidence retention behave as specified.

## Acceptance criteria

- The fixture suite fails note-only plan additions and accepts complete linked additions.
- Status-only output does not terminate an active run.
- Steering and retries are bounded and recorded.
- Failure, taint, rejection, and interruption are not classified as successful completion.

## Handoff

Hand off the fixture command, output, and coverage summary to W64/W65 and the release gate.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
