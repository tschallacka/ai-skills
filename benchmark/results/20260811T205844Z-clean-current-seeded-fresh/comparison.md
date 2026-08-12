# Batch overview

Total revisions benchmarked: 1. Completed successfully: 1. Tainted: 0. Lacked usable telemetry: 0.

This archive contains one current-protocol cohort, `reviewer-optimization-1.4.2`, with one revision: `current`. The selected plan directory named by `evaluation.md` is `basic-test-proof-current-20260811T205844Z-clean-current-seeded-fresh-isolated-plan`. The archive also contains top-level `planning/`, `provenance/`, and `reviewers/` directories; `planning/` is the required tagged skill archive, not counted as a worker plan.

# Revision comparison

| Revision | Protocol/cohort | Worker exit | Benchmark status | Validation result | Structural validation | Process audit | HTML/HTM audit | Session UUID | Telemetry records | Token total | Telemetry status | Tagged `planning/` skill | Accepted/tainted | Taint reasons | Approval/adoption contract |
|---|---|---:|---|---|---|---|---|---|---:|---:|---|---|---|---|---|
| `current` | `reviewer-optimization-1.4.2` | 0 | accepted | pass | pass | pass | 0 files | `019ff29e-d946-7402-9f70-1d63991c8774` | 1 | 3685989 | available from `threads.tokens_used` | present, 175 files | accepted; not tainted | none | `review_completed=true; plan_approved=false; oracle_completed=true; adoptable=false; semantic_true_positive_rate=0.0; independent_catch_rate=0.0; seeded_denominator=3; fail_closed_reasons=[APPROVAL_REJECTED,MISSING_DENOMINATOR,PLAN_NOT_APPROVED]; approval_conflict=false` |

# Deliverable inventory by revision

| Revision | Benchmark status | Selected plan directory | Goals | Work-unit inventory items | UI stories | UI run-cache items | Testing companions | Review report files | Review finding rows | Bug-register files | Bug-register rows | Context snapshots | Validation/analysis reports | Plan files | Result archive files | Telemetry records | Usage tokens | Integrity warning |
|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| `current` | accepted | `basic-test-proof-current-20260811T205844Z-clean-current-seeded-fresh-isolated-plan` | 2 | 7 | 1 | 1 | 7 | 1 | 1 | 1 | 0 | 1 | 2 | 29 | 267 | 1 | 3685989 | Top-level `planning/` is present and excluded from plan counts as the tagged skill archive; no extra candidate worker plan directory was merged. |

Counting rule: goals are `goal.md` files under the selected plan directory; work units are ID rows matching `W01`/`WU-01` style in `work-unit-inventory.md`; UI stories are `US-` rows and run-cache files; testing companions are `*-testing.md` step files; review report files and bug-register files are counted separately from review finding rows and bug rows. Ambiguous review rounds or fix cycles are reported as `not recorded` unless lifecycle or artifacts establish the boundary.

# Token usage progression

Only one revision in this batch has usable telemetry, so there is no previous usable-token revision for a delta. Comparison unavailable for `current` because token progression requires at least two usable-token revisions in semantic-version or harness-summary order.

Overall interpretation: observed usage for the single accepted run was 3,685,989 tokens. With no within-cohort predecessor in this batch, the archive supports no expansion/decrease trend and no claim that token usage changed because of planning-work size.

# Protocol 1.4.2 review and provenance outcomes

| Revision | Review mode | Reviewer sessions | Harness Reviewer A/B protocol sessions | Worker-internal reviewer activity | Cycles | Verification passes | Handoff/termination events | Independence status | Authority outcome | Schema outcome | Provenance outcome | Semantic oracle/transformation outcome | Final fresh-review approval | Unresolved limits |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| `current` | fresh-review | 3 records in telemetry summary/lifecycle | Reviewer B selected: `20260811T205844Z-clean-current-seeded-fresh-current-B-1786482768027444165`; no harness Reviewer A terminal approval | Two observed worker subagents: `019ff2a5-9895-7e12-b530-da42a17f1e7f`, `019ff2a8-2694-7e51-9f89-6acf39546ffa`; reported separately and not counted as harness Reviewer A/B approvals | Reviewer B cycle 1; worker-internal cycle 0 | Reviewer B verification pass 1 | Reviewer B launch then handoff; worker-internal launch and duplicate termination records observed | Reviewer B handoff records `independence=true`; worker-internal independence null/not applicable | passed: `reviewer_authority=reviewer-b` | passed: `approval_schema_status=valid` | passed binding/provenance evidence, hashes retained below | oracle completed; terminal accepted; semantic TPR 0.0, independent catch rate 0.0, mechanical exact-ID rate 0.0 | rejected: `overall_plan_approval=false`; `plan_approved=false` | fail-closed reasons retained as archived: `APPROVAL_REJECTED`, `MISSING_DENOMINATOR`, `PLAN_NOT_APPROVED`; `semantic_threshold=null`, `independent_threshold=null` |

Finding owners/closures: worker-internal artifacts mark `AR-01` resolved in the plan; the authoritative harness Reviewer B approval owns `AR-01`, `AR-02`, and `AR-03` as independent approved findings and closes with `overall_plan_approval=false`, so adoption remains closed rather than repaired from prose.

Current-protocol approval contract preserved from `evaluation.md`/`reviewer-state.json`:

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
    "APPROVAL_REJECTED",
    "MISSING_DENOMINATOR",
    "PLAN_NOT_APPROVED"
  ],
  "approval_conflict": false
}
```

Provenance hashes and archive paths retained:

| Revision | Record | Archive path | SHA-256 |
|---|---|---|---|
| `current` | source plan | `basic-test-proof-current-20260811T205844Z-clean-current-seeded-fresh-isolated-plan` | `5f231f106a6363a5f7a15674a06d9bb9f3299d0aee812dbfa96e002c8fca0f2d` |
| `current` | defective plan manifest | `provenance/defective-plan-manifest.json` | `af277111893fe9d0aa381243351cf8abbc6ebace9f107a0226b0b393bd9e1a21` |
| `current` | target snapshot manifest | `provenance/target-snapshot-manifest.json` | `af277111893fe9d0aa381243351cf8abbc6ebace9f107a0226b0b393bd9e1a21` |
| `current` | selected approval | `reviewers/20260811T205844Z-clean-current-seeded-fresh-current-B-1786482768027444165/plan/approval.json` | `e8a8f07eb972bd4bff1df4a9863c437bf21f9780a2c85aa6394d281b412db7f9` |
| `current` | selected capsule manifest | `reviewers/20260811T205844Z-clean-current-seeded-fresh-current-B-1786482768027444165/capsule-manifest.json` | `c80d7000e2c4d952829229035e4c87d21ed992a4cf2febd450b6e37b785df688` |
| `current` | transcript | `worker.jsonl` | `337771968600a3c46b20bb4b9d94bf63425fae3dfa50769ccc4b8af46aaf1be9` |
| `current` | lifecycle handoff | `reviewer-lifecycle.jsonl` | `247ee9d40d6fc7f254107fb4045e31d1724c1f2eb5ac03a0c16f3b0ede0c47f6` |
| `current` | binding | `reviewer-b-session-binding.json` | `ed93296e831a47ab742b6db75e3a0831f772a2b253b51597bac4f4c4b4a3d29e` |
| `current` | selection | `reviewer-selection.json` | `fd26c2580dbcade039fb92c608c0206547f691f6ef7cf5523decfd7a841312df` |

The machine-readable aliases `source_plan_sha256`, `defective_plan_sha256`, `target_snapshot_sha256`, `approval_sha256`, `transcript_sha256`, and `lifecycle_handoff_sha256` agree with the nested provenance records and `provenance_paths` in the archive.

# Developer journey by revision

## `current`

The worker approached the planning-only HTML task by reading the benchmark/task inputs and the tagged `planning/` skill, then decomposing the future `button-chain.html` behavior into two goals and seven work units: markup, style, append handler, finish handler, DOM proof, browser-story proof, and implementation acceptance review. The plan included one UI story and one run-cache item, both explicitly deferred because the benchmark forbade creating or testing the HTML file.

Review and fix cycles are split between worker-internal and harness evidence. The worker-internal artifacts record one stable adversarial finding, `AR-01`, about the off-by-one click sequence; the analysis report also describes a Reviewer B pass that found a goal-size mismatch after W07 was added, followed by a correction. Observable review rounds/fix cycles beyond those artifact statements are `not recorded` because the lifecycle does not provide a complete worker-internal round boundary.

The archived harness fresh Reviewer B session is authoritative for Protocol 1.4.2 and rejected final approval. It independently retained `AR-01` and added `AR-02` for a one-button/two-button initial-state contradiction and `AR-03` for white-border/black-border inconsistency; the blinded oracle accepted the review evidence but scored semantic true-positive and independent catch rates at 0.0 against three seeded items.

Final validation passed, structural validation passed, process audit passed, no HTML/HTM files were found, and UUID-matched telemetry was available. The notable constraint was the tagged helper rejecting uppercase `T`/`Z` in the requested directory name; the worker drafted under a lowercase helper-compatible name, renamed to the exact requested plan directory, and validated that final path.

# Access audit

The analyzer used the run archive under `/tmp/ai-skills-capsules/20260811T205844Z-clean-current-seeded-fresh/analysis/results`, including archived copies of `benchmark-test.md` and `harness-summary.tsv`. The named files under `/home/mdibbets/git/ai-skills/benchmark/results/20260811T205844Z-clean-current-seeded-fresh/analysis/` were not present in this sandbox, and no source checkout, installed skills, parent run archive, or prior benchmark archive was read.
