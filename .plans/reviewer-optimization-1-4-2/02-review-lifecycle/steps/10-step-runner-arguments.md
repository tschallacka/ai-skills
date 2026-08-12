# Step: 10-step-runner-arguments

## Ownership

- Goal: `02-review-lifecycle`
- Work unit: `W61`
- Type: `source`

## Change target

- File: `benchmark/planning/setup-and-run.sh`
- Primary symbol or file scope: `argument parser`
- Subscope: `N/A`

## Objective

§ 4.1
Accept the explicit --iterative/--fresh-review mode, --revisions tag list, run name, scheduling mode, and per-worker limits while rejecting malformed combinations before any process starts.

## Instructions

§ 5.1
Parse grammar `setup-and-run.sh <name> [--sequential|--parallel] [--iterative|--fresh-review] [--revisions tag[,tag...]]`; default mode is fresh-review and default scheduling is sequential. Reject duplicate modes, empty revision lists, malformed names, unknown options, and missing names before creating RUN_ID or starting a process. Export REVIEW_MODE, MAX_VERIFICATION_PASSES, and MAX_REVIEW_CYCLES into benchmark-env.sh and record them in harness-summary.tsv.

## Acceptance criteria

§ 6.1
The exact grammar accepts both pilot commands used by W36/W55, rejects unsupported legacy forms with exit 64 before process launch, and records normalized revisions, mode, limits, and run ID.

## Handoff

§ 7.1
Hand off normalized mode, comma-separated v-prefixed tags, defaults, limits, rejection exit codes, and harness-summary.tsv fields consumed by W36, W55, W59, and W38.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
