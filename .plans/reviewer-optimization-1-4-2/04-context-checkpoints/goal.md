# Goal: Add bounded context and phase checkpoint state

## Current state and prior-goal handoffs

§ 2.1
plan-context.sh and plan-context-lib.sh already provide bounded init/read/check/refresh behavior with hashes, snapshots, locks, and processed entries, but not source namespaces, phase views, per-worker variables, or checkpoint artifacts.

## Outcome and definition of done

§ 3.1
Integrate per-worker variables, bounded context snapshots, invalidation, checkpoints, and phase-specific reads without expanding counted plan deliverables.

## Why this goal is needed

§ 4.1
Reviewer token savings depend on compact, change-aware context rather than repeated whole-plan reads, while source and plan mutations must invalidate stale state.

## Scope

§ 5.1
Include source-aware indexing, fixed bounded views, isolated wrappers, phase checkpoints, and context tests. Exclude global IDs, history, quarantine, events, compaction, and workers explicitly deferred by the current skill contract.

## Affected files, systems, data, and interfaces

§ 6.1
Change planning/scripts/plan-context-lib.sh, planning/scripts/plan-context.sh, benchmark/planning/setup-benchmark.sh, worker-prompt.md, and planning/tests/test-plan-context.sh.

## Dependencies and handoffs

§ 7.1
Consume capsule paths from Goal 3 and the source-file contract from Goal 1. Hand off stable snapshot/checkpoint formats to archive, telemetry, and pilot verification.

## Implementation approach, risks, and edge cases

§ 8.1
Preserve existing bounded output and command contracts; add only phase-specific views and namespaced entries; invalidate memory on hash drift; serialize writes with existing locks; test parallel workers with distinct variable files.

## Owned work units

§ 9.1
`W16` — Add source namespaces for SKILL.md, REVIEWER.md, and approved relative references, retain hash freshness checks, and invalidate memory when sources or plan files change.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The goal changes executable benchmark or planning behavior and owns focused tests or verification units in the inventory. |

§ 9.2
`W17` — Expose bounded phase-specific summary, ownership/dependency, changed-document, and validator-focused views with fixed command contracts and size limits.

§ 9.3
`W18` — Generate isolated variables and a wrapper that sources them before invoking plan-context.sh, avoiding shared mutable variable files.

§ 9.4
`W19` — Require compact checkpoints after drafting, review, correction, and validation with hashes, changed files, open findings, and next action.

§ 9.5
`W20` — Verify bounded reads, changed-entry detection, source namespace freshness, invalidation after mutation, and no shared-state collision.

§ 9.6
`W47` — Persist compact phase checkpoints outside counted plan deliverables, with current state, open findings, next action, changed files, hashes, and source/plan invalidation.

§ 9.7
`W48` — Add quiet-by-default helper output, explicit verbose mode, size budgets for reports/companions/context, and bounded malformed-call retry messages.

§ 9.8
`W49` — Test checkpoint lifecycle, memory exclusion from counted deliverables, output budgets, quiet mode, and bounded retry behavior.

## Goal-size exception

§ 11.1
Not applicable: this goal owns multiple work units and does not use the single-unit exception.
