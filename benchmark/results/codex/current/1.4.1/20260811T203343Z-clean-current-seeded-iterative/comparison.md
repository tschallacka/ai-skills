# Batch overview

The batch `20260811T203343Z-clean-current-seeded-iterative` benchmarked 1 revision. Worker processes completed successfully by exit code: 1. Benchmark-accepted revisions: 0. Tainted revisions: 1. Revisions lacking usable telemetry: 0.

The only archived revision is `current`, and it belongs to the current-protocol cohort `reviewer-optimization-1.4.2`. No legacy cohort archive is present in this run, so all within-cohort metrics below are for the 1.4.2 cohort only.

## Revision comparison

| Revision | Protocol cohort | Worker exit | Benchmark status | Validation result | Structural validation | Process audit | HTML/HTM artifact audit | Session UUID | Telemetry records | Token total | Accepted/tainted | Taint reasons | Tagged `planning/` skill | Review mode | Reviewer sessions | Lifecycle / handoff / termination | Independence status | Final fresh-review approval | Approval contract |
|---|---|---:|---|---|---|---|---|---|---:|---:|---|---|---|---|---|---|---|---|---|
| `current` | `reviewer-optimization-1.4.2` | 0 | `tainted` | fail | pass | pass | `html_or_htm_files=0` | `019ff287-f36e-7ef3-9ddf-cac1274c692d` | 1 | 3024778 | tainted, gradeable but non-adoptable | `VALIDATION_FAILED`; fail-closed: `APPROVAL_REJECTED`, `MISSING_DENOMINATOR`, `PLAN_NOT_APPROVED`, `TAINTED_RUN` | present at `current/planning/` | iterative | worker-internal: `019ff28e-4ed6-7e33-a319-a57b8f370318`, `019ff292-431a-78e2-8129-63be182175f8`; Reviewer A: `20260811T203343Z-clean-current-seeded-iterative-current-A-1786481374515110835`; Reviewer B: `20260811T203343Z-clean-current-seeded-iterative-current-B-1786481534995424311` | worker-internal launches and duplicate terminations observed; Reviewer A launch then handoff; Reviewer B launch then handoff with selected approval | Reviewer A `independence=false` and handoff-only; Reviewer B `independence=true`; worker-internal independence not applicable | Reviewer B terminal approval selected, `overall_plan_approval=false` | `{"review_completed":true,"plan_approved":false,"oracle_completed":true,"adoptable":false,"semantic_true_positive_rate":0.0,"independent_catch_rate":0.0,"seeded_denominator":3,"fail_closed_reasons":["APPROVAL_REJECTED","MISSING_DENOMINATOR","PLAN_NOT_APPROVED","TAINTED_RUN"],"approval_conflict":false}` |

### Protocol 1.4.2 authority, schema, provenance, and oracle outcomes

| Revision | Authority outcome | Schema outcome | Provenance outcome | Semantic oracle / transformation status | Mechanical exact-ID diagnostics | Reviewer binding | Approval schema | Unresolved limits |
|---|---|---|---|---|---|---|---|---|
| `current` | Selected authority is Reviewer B; no unauthorized approvals recorded; Reviewer A is handoff-only and does not make approval true. | `approval_schema_status=valid`; schema validation covers required metadata, types, complete finding objects, and duplicate finding IDs. | `reviewer_binding_status=passed`; retained hashes and paths are listed below. | Blinded oracle accepted the archive as terminal; semantic TPR `0.0`, independent catch rate `0.0`, seeded denominator `3`; selected Reviewer B rejected the plan. | exact ID matches `0`; exact ID rate `0.0`; reported separately from semantic rates. | `passed` | `valid` | Thresholds are not recorded: `semantic_threshold=null`, `independent_threshold=null`; fail-closed includes `MISSING_DENOMINATOR` as archived. |

Retained provenance for `current`:

| Provenance item | Archive path | SHA-256 |
|---|---|---|
| Source plan | `basic-test-proof-current-20260811T203343Z-clean-current-seeded-iterative-isolated-plan` | `1e7937d119b307d10bb09c51e5afd5b3fcec3cbf31a2550d17d5a5c2e2148e14` |
| Defective-plan manifest | `provenance/defective-plan-manifest.json` | `d05a5ca423f8fef2fd029f17ace5c2b60d29b5ef0420ed26a837658b1e689cb5` |
| Target snapshot manifest | `provenance/target-snapshot-manifest.json` | `d05a5ca423f8fef2fd029f17ace5c2b60d29b5ef0420ed26a837658b1e689cb5` |
| Selected approval | `reviewers/20260811T203343Z-clean-current-seeded-iterative-current-B-1786481534995424311/plan/approval.json` | `e131166a9e967ad170c8d7408c57c6673283518334c06d9e997faca5093587c4` |
| Selected capsule manifest | `reviewers/20260811T203343Z-clean-current-seeded-iterative-current-B-1786481534995424311/capsule-manifest.json` | `bcebe47f9b7f09c92314f980912c3c448591c7f726d50a25913d85e8b343870c` |
| Transcript | `worker.jsonl` | `21dfdb61a695a078cb7aeb06bebdfd0da38ea301bc94af55cb8735a402f0ce38` |
| Lifecycle handoff | `reviewer-lifecycle.jsonl` | `2be1c9b224dc6b7e87f0610b450e737eeb93d79964f870525eb60a1ec208383a` |
| Binding record | `reviewer-b-session-binding.json` | `da32d0a4da39f6d1f5e70bb6069ee84358b726152e93ec6fd9b81ad94d691be9` |

The machine-readable aliases in `reviewer-state.json`, `protocol-metadata.json`, and `telemetry.json` agree with the nested provenance records and `provenance_paths` for `source_plan_sha256`, `defective_plan_sha256`, `target_snapshot_sha256`, `approval_sha256`, `transcript_sha256`, and `lifecycle_handoff_sha256`.

## Deliverable inventory by revision

Selected plan directory for `current`: `basic-test-proof-current-20260811T203343Z-clean-current-seeded-iterative-isolated-plan`.

| Revision | Benchmark status | Goals | Work-unit inventory items | UI story items | UI run-cache items | Testing companions | Review report files | Review finding entries | Bug-register files | Bug entries | Context snapshots | Validation reports | Analysis reports | Total selected-plan files | Total result-archive files | Telemetry records | Total usage tokens | Integrity warning |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| `current` | tainted | 2 | 7 | 1 | 1 | 7 | 1 | 2 | 1 | 0 | 1 | 1 | 1 | 27 | 296 | 1 | 3024778 | No extra top-level candidate plan directories observed outside the selected plan; archived tagged `planning/` contains historical `plans/` examples and is excluded from selected-plan counts. |

Counting rule: goals are `goal.md` files under first-level goal directories in the selected plan; work units are ID rows in `work-unit-inventory.md` matching `W01`, `W02`, etc.; UI stories are story rows in `ui-user-stories.md`; run-cache items are files under `ui-story-runs/`; testing companions are `*-testing.md` files under goal `steps/`; review report files count `adversarial-review.md` and review finding entries count AR rows/objects separately; bug-register files count `bugs.md` and bug entries count data rows separately; validation and analysis reports count `validation.md` and `analysis-report.md`.

## Token usage progression

| Revision | Previous usable-token revision | Previous tokens | Current tokens | Delta | Change | Interpretation |
|---|---|---:|---:|---:|---|---|
| `current` | none | n/a | 3024778 | n/a | baseline | First usable-token revision in this batch; comparison unavailable because there is no prior revision in the archive. |

Overall interpretation: token usage for the batch is a single observed baseline, not a progression. The `current` run spent 3,024,778 tokens while producing a 27-file selected plan, two protocol reviewer sessions, two worker-internal reviewer-like sessions, validation evidence, and a terminal rejected approval; with no adjacent revision in this batch, no increase or decrease can be attributed or even compared.

## Developer journey by revision

### `current`

The worker began by recording the session UUID from `CODEX_THREAD_ID`, reading the benchmark task, reading the tagged local `planning/` skill and its UI validation reference, and creating a planning-only directory for the future `button-chain.html` task. The plan decomposed the future work into two goals and seven work units: markup, completion styling, append behavior, fourth-generated-button finish behavior, readiness verification, browser-story verification, and artifact audit.

The observable review path has worker-internal subagent activity, followed by protocol Reviewer A and Reviewer B lifecycle records. The worker report says the first fresh review returned four findings and that corrective edits were applied for placeholder cleanup, UI story cache specificity, and testing companion specificity; the exact number of correction/fix cycles is not recorded as a bounded lifecycle count. Reviewer B then produced a terminal, schema-valid approval artifact with two approved findings and `overall_plan_approval=false`.

The plan strengthened the target contract after review: the final plan description now requires one initial button, current-last-button appends, fourth generated button completion, exact lowercase `finished`, and a visible white border. Final validation failed with three retained gate failures: adversarial review not approved, plan-description review status not mirroring approval, and unresolved adversarial-review findings. The archive shows no HTML/HTM artifacts, process audit passed, structural validation passed, telemetry was available, and the run is tainted only by `VALIDATION_FAILED` while remaining gradeable non-adoption evidence.
