# Batch overview

Total revisions benchmarked: 1. Worker-completed successfully: 1 (`exit_code=0`). Tainted revisions: 1. Revisions lacking usable telemetry: 0. Accepted, untainted revisions: 0.

This run contains one 1.4.2-cohort archive (`protocol_id=reviewer-optimization-1.4.2`). The legacy/cross-cohort comparison set is empty in this batch, so all metrics below are within the 1.4.2 cohort only.

| Revision | Benchmark status | Worker exit status | Validation result | HTML/HTM artifact audit | Session UUID | Telemetry records | Token total | Tagged `planning/` skill | Accepted/tainted status | Taint reasons |
|---|---|---:|---|---|---|---:|---:|---|---|---|
| `1.3.1` | tainted | 0 | pass; structural validation pass; process audit pass | 0 `.html`/`.htm` files | `019fefb9-51fb-7c22-b039-2f8f12ce0557` | 1 | 2430407 | present at `planning/SKILL.md` | tainted | `REVIEWER_LIFECYCLE_FAILED` in `reviewer-lifecycle.jsonl` |

# Deliverable inventory by revision

| Revision | Benchmark status | Goals | Work-unit inventory items | UI story items | UI run-cache files / rows | Testing companions | Review report files | Review findings | Bug-register files | Bug entries | Context snapshots | Validation/analysis reports | Selected plan files | Result archive files | Telemetry records | Usage tokens |
|---|---|---:|---:|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `1.3.1` | tainted | 2 | 5 | 1 | 1 file / 10 rows | 5 | 1 | 1 | 1 | 0 | 1 | 2 | 24 | 93 | 1 | 2430407 |

Counting rule: selected plan counts use only the plan directory named in `evaluation.md`, `basic-test-proof-1.3.1-20260811T072847Z-pilot-142-fresh-131-restart4-isolated-plan`. Work units are counted from ID rows in `work-unit-inventory.md` matching `WU-01`, `W01`, etc.; review report files and bug-register files are counted separately from `AR-*` review findings and `BUG-*` bug entries. UI run-cache rows count numbered interaction/readiness rows in `ui-story-runs/US-01.md`. The reviewer snapshot under `reviewers/.../plan` is reviewer evidence and is excluded from selected-plan counts; no extra root-level candidate plan directory was counted.

# Protocol 1.4.2 lifecycle evidence

| Revision | Review mode | Harness reviewer sessions | Worker-internal reviewer activity | Cycles | Verification passes | Finding owners/closures | Handoff and termination events | Independence status | Unresolved limits | Final fresh-review approval |
|---|---|---|---|---|---|---|---|---|---|---|
| `1.3.1` | `fresh-review` | Reviewer B session `20260811T072847Z-pilot-142-fresh-131-restart4-1.3.1-B-1786434174934363389`; launch and handoff recorded | Two worker-internal subagent session IDs observed; each has launch and duplicate termination records; reported separately and not counted as harness Reviewer A/B sessions | Harness cycle 1 recorded; worker-internal cycle 0 observed | Harness verification pass 1 recorded; worker-internal pass 0 observed | Final `approval.json` approves `AR-01`; protocol ownership/closure fields are otherwise unavailable in lifecycle JSONL | Reviewer B `launch` and `handoff` recorded with `exit_code=65`; worker-internal `observed-launch` and `observed-termination` recorded | Reviewer B handoff has `independence=true`; worker-internal independence is `null` | Reviewer lifecycle status failed, causing taint; duplicate worker-internal termination records are observable | `overall_plan_approval=true`, approved at `2026-08-11T07:44:32Z` |

# Token usage progression

| Revision | Previous revision | Previous tokens | Current tokens | Delta | Change | Interpretation |
|---|---|---:|---:|---:|---|---|
| `1.3.1` | none | unavailable | 2430407 | comparison unavailable | comparison unavailable | First usable-token revision in the batch; no prior revision exists for a within-batch progression comparison. |

Overall interpretation: no token expansion or decrease can be measured across this batch because only one revision has usable telemetry. The observed 2430407-token total corresponds to a planning run that produced two goals, five work units, five testing companions, UI story/run-cache evidence, review/bug/context/validation/analysis artifacts, and reviewer lifecycle evidence; this is an observed usage profile, not causal proof of token cost drivers.

# Developer journey by revision

## `1.3.1`

The worker started by recording `session-id.txt` from `CODEX_THREAD_ID`, then read the benchmark task and tagged `planning/` skill inputs before creating a planning-only durable plan for the future `button-chain.html` behavior. The selected plan decomposed the task into two goals and five work units: markup, completion styling, click behavior, automated future verification, and browser-story future verification.

Observable review rounds and fix cycles are partly recorded in plan prose but not fully protocol-owned in the lifecycle artifact: the analysis report records Reviewer A initially finding five open issues, corrections to trackers, testing companions, UI run-cache rows, related work-unit coverage, and the W04 verification plan, then final independent approval; exact harness Reviewer A cycles are not recorded. The 1.4.2 lifecycle artifact records one harness Reviewer B fresh-review cycle with verification pass 1 and `independence=true`, plus two worker-internal reviewer subagent sessions kept separate from protocol Reviewer A/B ownership.

After review, the plan was strengthened with plan-level and goal-level progress trackers, five testing companion files, one UI story, one detailed browser run cache with 10 counted rows, a bug-register placeholder with no bug entries, and final validation/analysis reports. Final validation passed (`Plan validation passed: 5 work units across 2 goals`), structural validation passed, process audit passed, and the HTML/HTM audit found zero files. The notable failure is not worker output quality but protocol evidence: `REVIEWER_LIFECYCLE_FAILED` taints the run even though final fresh-review approval is present.
