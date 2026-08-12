# Batch overview

Batch `20260811T152024Z-old-plan-iterative` benchmarked 1 revision: `current`. The worker completed successfully for 1 revision, 0 revisions are marked tainted by the harness evaluation, and 0 revisions lack usable telemetry. The archive belongs to the `reviewer-optimization-1.4.2` cohort; there are no legacy cohort rows in this batch.

The selected plan directory from `evaluation.md` is `.plans/basic-test-proof-current-20260811T152024Z-old-plan-iterative-isolated-plan`. No extra worker candidate plan directories were present under `.plans`; archived provenance examples under `planning/plans/*` are part of the tagged `planning/` skill copy and are excluded from plan counts.

## Deliverable inventory by revision

| Revision | Benchmark status | Worker exit | Validation result | HTML/HTM audit | Session UUID | Telemetry records | Usage tokens | Accepted/tainted | Taint reasons | Tagged `planning/` skill | Protocol/cohort | Review mode | Reviewer protocol sessions | Worker-internal reviewer sessions | Cycles / verification passes | Independence status | Handoff / termination events | Final fresh-review approval contract | Goals | Work-unit items | UI story / run-cache items | Testing companions | Review report files / findings | Bug-register files / bug entries | Context snapshots | Validation / analysis reports | Plan files | Result archive files |
|---|---:|---:|---|---:|---|---:|---:|---|---|---|---|---|---|---|---|---|---|---|---:|---:|---|---:|---|---|---:|---:|---:|---:|
| `current` | accepted | 0 | pass; structural pass | 0 | `019ff169-1ae4-75b1-a3ce-bfc833a8fd76` | 1 | 2,723,778 | accepted | none | present | `reviewer-optimization-1.4.2` / `1.4.2` | iterative | Reviewer A `20260811T152024Z-old-plan-iterative-current-A-1786462220720411889`; Reviewer B `20260811T152024Z-old-plan-iterative-current-B-1786462400350746198` | `019ff16e-539a-7892-b17f-83d1c0f25b87` | cycle 1; verification pass 1 for A and B | A `false`; B `true` | A handoff; B handoff; worker-internal launch and duplicated termination record | review completed, oracle completed, `plan_approved=false`, `adoptable=false`, conflict true | 2 | 6 | 1 story / 1 run cache | 6 | 1 / 0 selected-plan findings; protocol A recorded 1 finding | 1 / 1 placeholder row, 0 open bugs | 1 | 2 | 27 | 286 |

Counting rule: goals are selected-plan `goal.md` files; work units are `W01`-style ID rows in `work-unit-inventory.md`; UI stories and run-cache items are selected-plan `US-01` rows/files; testing companions are `*-testing.md` files; review report files are counted separately from finding rows; bug-register files are counted separately from bug rows and open bug count; validation/analysis reports count selected-plan `validation.md` and `analysis-report.md`; result archive file count is for the per-revision archive `results/current` and excludes run-level files such as this `comparison.md`.

### 1.4.2 approval state

The current-protocol machine-readable state is preserved from `evaluation.md`, `reviewer-state.json`, and `protocol-metadata.json`:

```json
{
  "review_completed": true,
  "plan_approved": false,
  "oracle_completed": true,
  "adoptable": false,
  "semantic_true_positive_rate": 0.0,
  "independent_catch_rate": 0.0,
  "seeded_denominator": 3,
  "fail_closed_reasons": [
    "APPROVAL_CONFLICT",
    "INDEPENDENT_THRESHOLD_FAILED",
    "PLAN_NOT_APPROVED",
    "SEMANTIC_THRESHOLD_FAILED"
  ],
  "approval_conflict": true
}
```

Mechanical exact-ID diagnostics are separate from the semantic approval state: `mechanical_exact_id_matches=0` and `mechanical_exact_id_rate=0.0`. The blinded oracle completed with `terminal=true`, seeded denominator `3`, true positives `0`, false negatives `3`, false positives `4`, independent catches `0`, and no unresolved or ambiguous findings.

## Token usage progression

| Revision | Previous revision | Previous tokens | Current tokens | Delta | Change | Interpretation |
|---|---|---:|---:|---:|---|---|
| `current` | none | unavailable | 2,723,778 | comparison unavailable | comparison unavailable | This is the first and only usable-token revision in the batch, so no within-cohort progression can be computed. |

Overall interpretation: token usage for `current` is high at 2,723,778 recorded tokens, but with no prior usable-token revision in this batch there is no observable increase or decrease trend. The inventory shows a complete two-goal, six-work-unit planning proof with worker-internal review plus protocol A/B review evidence; that describes the observed work volume, not a causal explanation for the token total.

## Developer journey by revision

### `current`

The worker approached the task as a planning-only proof: it wrote `session-id.txt`, read the benchmark task and tagged `planning/` skill, created the selected `.plans/...isolated-plan` directory, and decomposed the future `button-chain.html` task into two goals and six work units. The plan includes one UI story and one cached browser run sequence, but no HTML was created or tested; the HTML/HTM artifact audit found 0 files.

Observable review activity has three distinct layers. Inside the worker session, one worker-internal Reviewer B subagent reviewed and approved the plan, wrote `approval.json`, and updated `adversarial-review.md`; review rounds and fix cycles before that internal approval are not fully bounded, so the count is `not recorded`. The protocol lifecycle then recorded Reviewer A and Reviewer B in iterative mode, both at cycle 1 / verification pass 1; Reviewer A was not independent and recorded AR-01 against the top-level desired outcome, while Reviewer B was independent and ended with `overall_plan_approval=false`.

The worker strengthened the plan after review by aligning the final plan artifacts around one initial button, current-last-button appends, fourth generated-button completion, exact lowercase `finished`, and a visible white border. It also recorded two helper deviations in `context-snapshot.md`: the create helper rejected the mixed-case benchmark directory name before the plan was moved to the required name, and the work-unit update helper corrupted two inventory rows that were repaired in the selected plan.

Final validation passed with `Plan validation passed: 6 work units across 2 goals`, structural validation passed, process audit passed, and UUID-matched telemetry was available. The notable failure is protocol-level, not worker-exit-level: the 1.4.2 approval state is non-adoptable because `PLAN_NOT_APPROVED`, `APPROVAL_CONFLICT`, `SEMANTIC_THRESHOLD_FAILED`, and `INDEPENDENT_THRESHOLD_FAILED` are all recorded.
