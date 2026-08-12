# Goal 03 / Step 05: Verify the full current-protocol release gate

## Ownership

- Goal: `03-regression-and-release-gates`
- Work unit: `W11`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `current-protocol release gate`
- Subscope: `adoption-state output`

## Objective

Exercise the real worker → Reviewer B → independent oracle → analyzer → archive
path and verify the final decision.

## Instructions

Run exactly the command in plan §8.7 with
`BLINDED_ORACLE_SPEC=benchmark/planning/pilot-blinded-defects.json` and the
resource-limited wrapper. Record the emitted run ID and inspect only that run.
Require
worker completion, fresh Reviewer B lifecycle evidence, oracle adjudication,
telemetry, archive publication, analyzer comparison, and final validators.
Simulation-only completion is insufficient. Historical reports remain context.

## Acceptance criteria

- AR-01 evidence is scored as three semantic true positives.
- False approval or below-threshold rates keep adoption false.
- Every worker, oracle, analyzer, archive, and plan validator check passes.

## Handoff

Record commands, archive paths, metrics, and remaining adoption gates in the
Goal 03 working context before marking completion.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
