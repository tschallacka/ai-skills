# Goal: Implement bounded iterative reviewer lifecycle

## Current state and prior-goal handoffs

§ 2.1
The runner currently launches workers and one analyzer, with signal cleanup and summary output but no iterative reviewer state machine or reviewer lifecycle records.

## Outcome and definition of done

§ 3.1
Add explicit iterative-review execution, fresh-review fallback, pass/cycle limits, finding ownership, and handoff semantics to the benchmark prompts and runner.

## Why this goal is needed

§ 4.1
The token-saving behavior is only safe if correction passes are bounded and a fresh reviewer still performs the final independent review.

## Scope

§ 5.1
Include runner lifecycle state, worker/analyzer prompt contracts, option/limit documentation, and bounded harness tests. Exclude filesystem capsule construction and telemetry extraction.

## Affected files, systems, data, and interfaces

§ 6.1
Change benchmark/planning/run-benchmark.sh, worker-prompt.md, analyzer-prompt.md, benchmark-test.md, and the focused runner test fixture.

## Dependencies and handoffs

§ 7.1
Consume Goal 1 field and rule names. Hand off lifecycle event shapes and mode defaults to capsule, archive, analyzer, and pilot work.

## Implementation approach, risks, and edge cases

§ 8.1
Keep fresh-review mode as the default; bound verification passes and total fresh-review cycles; reject self-approval of the overall plan; terminate a reviewer that exceeds its limit and start a fresh one.

## Owned work units

§ 9.1
`W06` — Add bounded review-cycle state, reviewer process tracking, signal-safe cancellation, fresh-session spawning, pass/cycle limits, and lifecycle event recording.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The goal changes executable benchmark or planning behavior and owns focused tests or verification units in the inventory. |

§ 9.2
`W07` — Specify Reviewer A finding ownership, bounded verification passes, concise handoff/termination, and fresh reviewer replacement rules while preserving fresh-review default behavior.

§ 9.3
`W08` — Require separate reporting of review cycles, verification passes, termination/handoff events, independence status, and unresolved limits.

§ 9.4
`W09` — Document iterative mode inputs, hard limits, default-mode compatibility, and acceptance criteria for Reviewer B independent defect detection.

§ 9.5
`W10` — Exercise option validation, fresh-review fallback, maximum-pass termination, and lifecycle metadata in a bounded harness fixture.

§ 9.6
`W39` — Forward Ctrl+C/TERM to every active worker, reviewer, analyzer, and child process; wait for cleanup, remove temporary state, and return a distinct interrupted status.

§ 9.7
`W40` — Own reviewer session/cycle state, launch fresh reviewer sessions, enforce pass/cycle limits, and persist lifecycle records separately from worker/analyzer execution.

§ 9.8
`W41` — Write changed-file/diff/targeted-validation handoffs, stable AR-NN ownership, closure passes, termination events, and final fresh-review approval artifacts.

§ 9.9
`W42` — Test option validation, Reviewer A ownership limits, fresh Reviewer B isolation, handoff artifacts, final approval prohibition, and interruption propagation.

§ 9.10
`W61` — Accept the explicit --iterative/--fresh-review mode, --revisions tag list, run name, scheduling mode, and per-worker limits while rejecting malformed combinations before any process starts.

## Goal-size exception

§ 11.1
Not applicable: this goal owns multiple work units and does not use the single-unit exception.
