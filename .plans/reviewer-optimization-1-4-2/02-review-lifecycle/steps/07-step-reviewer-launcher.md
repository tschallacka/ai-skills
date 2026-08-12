# Step: 07-step-reviewer-launcher

## Ownership

- Goal: `02-review-lifecycle`
- Work unit: `W40`
- Type: `source`

## Change target

- File: `benchmark/planning/run-benchmark.sh`
- Primary symbol or file scope: `reviewer launch block`
- Subscope: `N/A`

## Objective

§ 4.1
Own reviewer session/cycle state, launch fresh reviewer sessions, enforce pass/cycle limits, and persist lifecycle records separately from worker/analyzer execution.

## Instructions

§ 5.1
In run-benchmark.sh, implement a `launch_reviewer()` block that invokes `setsid timeout "$REVIEWER_TIMEOUT" codex -a never exec --json -C "$REVIEWER_WORKSPACE" --skip-git-repo-check --sandbox workspace-write --add-dir "$REVIEWER_CAPSULE" --add-dir "$REVIEWER_WORKSPACE" "$REVIEWER_PROMPT"`. Reviewer A receives `changed-files.txt`, `bounded.diff`, and `targeted-validation.txt`; Reviewer B receives a newly built full-review capsule and never receives A handoff conclusions. Persist `reviewer-lifecycle.jsonl` with session, cycle, pass, owner, closure, handoff, and termination events.

## Acceptance criteria

§ 6.1
A bounded fixture shows A receives only correction inputs, B receives a full fresh-review input set, A cannot approve the plan, B has a distinct session/capsule, and the runner records the lifecycle events.

## Handoff

§ 7.1
Record the changed target, command/output evidence, and unavailable or environment-specific results for the next dependency.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
