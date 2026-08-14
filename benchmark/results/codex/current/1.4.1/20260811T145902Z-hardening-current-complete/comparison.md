# Batch overview

Benchmarked revisions: 1. Completed successfully: 1. Tainted revisions: 0. Revisions lacking usable telemetry: 0.

This batch contains one observable revision archive, `current`, and it is a `reviewer-optimization-1.4.2` fresh-review cohort. No legacy cohort archives were present in the current run results. The analysis-root `benchmark-test.md` and `harness-summary.tsv` paths named in the prompt were not present at `/home/mdibbets/git/ai-skills/benchmark/results/20260811T145902Z-hardening-current-complete/analysis/`; the same records were observable inside the current run archive and were used.

## Revision comparison

| Revision | Cohort / mode | Worker exit | Validation | Structural validation | HTML/HTM audit | Session UUID | Telemetry records | Token total | Benchmark status | Taint reasons | Review lifecycle | Approval / adoption state |
|---|---|---:|---|---|---:|---|---:|---:|---|---|---|---|
| `current` | `reviewer-optimization-1.4.2` / `fresh-review` | 0 | pass | pass | 0 | `019ff155-8d55-7d91-b7d5-bd770254e924` | 1 | 2,181,243 | accepted | none | Reviewer B harness session `20260811T145902Z-hardening-current-complete-current-B-1786460818027269162`; cycle 1; verification pass 1; launch and handoff recorded; independence `true`; termination event not separately recorded beyond handoff exit code 0; finding owners/closures not recorded in lifecycle; unresolved limits: semantic_threshold `null`, independent_threshold `null` | `review_completed=true`, `plan_approved=false`, `oracle_completed=true`, `adoptable=false`, `semantic_true_positive_rate=0.0`, `independent_catch_rate=0.0`, `seeded_denominator=3`, `fail_closed_reasons=["MISSING_DENOMINATOR","PLAN_NOT_APPROVED"]`, `approval_conflict=false` |

Current-protocol approval contract preserved from `reviewer-state.json` / `protocol-metadata.json`:

```json
{
  "review_completed": true,
  "plan_approved": false,
  "oracle_completed": true,
  "adoptable": false,
  "semantic_true_positive_rate": 0.0,
  "independent_catch_rate": 0.0,
  "seeded_denominator": 3,
  "fail_closed_reasons": ["MISSING_DENOMINATOR", "PLAN_NOT_APPROVED"],
  "approval_conflict": false
}
```

The exact tagged `planning/` skill is present in the revision archive at `current/planning/`; this run does not predate the archive requirement.

## Deliverable inventory by revision

| Revision | Benchmark status | Selected plan directory | Goals | Work-unit inventory rows | UI story docs | UI story/run-cache files | Testing companions | Review report files | Review findings | Bug-register files | Bug entries | Context snapshots | Validation/analysis reports | Approval files | Total plan files | Total revision archive files | Telemetry records | Total usage tokens | Integrity warning |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| `current` | accepted, untainted, non-adoptable | `basic-test-proof-current-20260811T145902Z-hardening-current-complete-isolated-plan` | 2 | 6 | 1 | 1 | 6 | 1 | 1 | 1 | 0 | 1 | 2 | 1 | 27 | 256 | 1 | 2,181,243 | Tagged skill provenance includes six plan-like directories under `planning/plans/`; they were excluded from all selected-plan counts. No extra top-level worker plan directory was present. |

Counting rule: goal count is `goal.md` files under the selected plan directory named by `evaluation.md`; work units are ID rows matching `W01`, `WU-01`, etc. in `work-unit-inventory.md`; UI story docs count `ui-user-stories.md` and run-cache files count files under `ui-story-runs/`; testing companions are `*-testing.md`; review report files count `adversarial-review.md` as a report, while review findings count `AR-*` rows separately; bug-register files count `bugs.md`, while bug entries count `BUG-*` rows separately; validation/analysis reports count `validation.md` and `analysis-report.md`. Review rounds and fix cycles are `not recorded` unless bounded by lifecycle records or explicit artifact state.

## Token usage progression

| Revision | Previous revision | Previous tokens | Current tokens | Delta | Change | Interpretation |
|---|---|---:|---:|---:|---:|---|
| `current` | comparison unavailable | unavailable | 2,181,243 | comparison unavailable | comparison unavailable | This is the first and only usable-token revision in the batch, so there is no within-batch predecessor for the required progression formula. |

Overall interpretation: token expansion or decrease cannot be established across this batch because only one revision has usable telemetry. The single observed total is an exact UUID-matched telemetry value for the worker thread, but it is not comparative evidence by itself.

## Developer journey by revision

### `current`

- The worker treated the task as a planning-only proof, read the benchmark and task spec, then used the tagged `planning/` skill and its UI user-story validation reference rather than creating or testing `button-chain.html`.
- It built a durable plan with two goals: one for future implementation of root markup, completion styling, append behavior, and build review, and one for future DOM and browser-story verification. The selected work-unit inventory contains six ID rows, including a dedicated build-goal review unit and a browser flow `US-01` with a saved run cache.
- Observable review rounds: harness lifecycle records one fresh Reviewer B cycle and one verification pass; worker-internal structural review records one finding, `AR-01`. Fix cycles: not recorded as bounded cycles, but the artifact trail shows `approval.json` was added and the finding marked resolved while preserving `overall_plan_approval=false`.
- Final evidence is strong on structural completion: tagged validation passed, harness structural validation passed, process audit passed, and the archive contains no HTML/HTM artifacts. The notable constraint is adoption: the blinded oracle accepted the benchmark output, but the 1.4.2 approval state remains non-adoptable with `PLAN_NOT_APPROVED` and `MISSING_DENOMINATOR`; no access escape events were observed in the worker analysis report.
