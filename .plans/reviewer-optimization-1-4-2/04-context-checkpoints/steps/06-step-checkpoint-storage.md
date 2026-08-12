# Step: 06-step-checkpoint-storage

## Ownership

- Goal: `04-context-checkpoints`
- Work unit: `W47`
- Type: `source`

## Change target

- File: `planning/scripts/plan-context-lib.sh`
- Primary symbol or file scope: `checkpoint persistence`
- Subscope: `N/A`

## Objective

§ 4.1
Persist compact phase checkpoints outside counted plan deliverables, with current state, open findings, next action, changed files, hashes, and source/plan invalidation.

## Instructions

§ 5.1
Use the checkpoint schema defined in the plan description. The named callers are setup-benchmark.sh after worker draft and final validation, run-benchmark.sh after Reviewer A review and correction, and plan-context.sh check --changed for stale detection. Write /tmp/ai-skills-checkpoints/<run-id>/<revision>/<phase>.json through .tmp plus rename and reject mismatched identity or source/plan hashes.

## Acceptance criteria

§ 6.1
open_findings items validate as typed AR records; the checkpoint command, four caller invocations, stale/identity rejection JSON, exit 65, atomic temp/rename, and phase paths are all independently tested.

## Handoff

§ 7.1
Record the changed target, command/output evidence, and unavailable or environment-specific results for the next dependency.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
