# Batch overview

Total revisions benchmarked: 1. Worker-completed successfully: 1. Tainted benchmark results: 1. Revisions lacking usable telemetry: 0.

This run is a distinct Protocol 1.4.2 cohort (`protocol_id=reviewer-optimization-1.4.2`). There are no legacy revisions in this batch, so no cross-cohort metric comparison is made.

## Revision comparison

| Revision | Benchmark status | Worker exit | Validation result | Structural validation | Process audit | HTML/HTM artifact audit | Session UUID | Telemetry records | Token total | Accepted/tainted | Taint reasons | Archived tagged `planning/` skill | Protocol/review lifecycle |
|---|---:|---:|---|---|---|---|---|---:|---:|---|---|---|---|
| `current` | completed, tainted | 0 | pass (`Plan validation passed: 5 work units across 1 goals.`) | pass | pass | `html_or_htm_files=0`; no `.html`/`.htm` files observed in archive | `019ff0a1-fe08-7c12-9265-11cda4fe3203` | 1 | 2649339 | tainted | `REVIEWER_LIFECYCLE_FAILED`; `BLINDED_ORACLE_FAILED` (`oracle-rejection.json`: blinded defect seeding failed) | present at `current/planning/SKILL.md` | `fresh-review`; lifecycle status failed. Harness reviewer evidence: Reviewer B session `20260811T114255Z-current-fresh4-current-B-1786449342988261544`, cycle 1, verification pass 1, handoff event, `exit_code=65`, independence `true`. Worker-internal reviewer subagents observed separately: `019ff0a6-e1af-7650-924b-c5d7a2438747` and `019ff0a9-dc9e-7ef3-87bf-4169273c8fc6`, actor `worker-subagent`, protocol role `worker-internal`, launch/termination events recorded. Final plan artifact records fresh review cycle 2 approved, owner Reviewer B, closed findings `[]`, no pending findings. |

# Deliverable inventory by revision

| Revision | Benchmark status | Selected plan directory | Integrity warning | Goals | Work-unit inventory items | UI story items | UI run-cache items | Testing companions | Review report files | Review findings | Bug-register files | Bug entries | Context snapshots | Validation/analysis reports | Plan directory files | Result archive files | Telemetry records | Total usage tokens |
|---|---|---|---|---:|---:|---:|---:|---:|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|
| `current` | completed, tainted | `basic-test-proof-current-20260811T114255Z-current-fresh4-isolated-plan` | none for deliverable plans; root `planning/` is the archived tagged skill and excluded from plan counts | 1 | 5 | 1 | 1 | 5 | 1 | 0 final; prior AR-01..AR-03 noted as closed by overwrite history, not counted as final findings | 1 | 0 | 1 | 2 | 22 | 243 | 1 | 2649339 |

Counting rule: counts use only observable files in the revision archive and the plan directory named by `evaluation.md`. Work units are counted from ID rows in `work-unit-inventory.md` (`W01` through `W05`), UI stories from ID rows in `ui-user-stories.md`, run-cache items from files under `ui-story-runs/`, testing companions from `*-testing.md`, review report files from `adversarial-review.md`, bug-register files from `bugs.md`, and bug entries from numbered bug rows in that register. Review report files and bug-register files are file counts, separate from review findings or bug entries.

# Token usage progression

Only one revision in this batch has usable telemetry, so there is no prior usable-token revision for a within-batch delta. The `current` revision has 2649339 total usage tokens from 1 UUID-matched telemetry record.

Comparison unavailable for `current`: no previous revision exists in this batch. The observed token total accompanies a relatively expansive planning artifact set: one goal, five atomic work units, one UI story plus run cache, five testing companions, explicit bug recovery, context, validation, analysis, and a fresh-review approval artifact. That is an observed usage/work-inventory relationship only; token totals alone do not prove causation.

# Developer journey by revision

## `current`

The worker treated the task as a planning-only proof for a future `button-chain.html` implementation and kept the actual HTML uncreated. It read the tagged planning skill, created a selected plan directory with one goal and five work units covering markup, append behavior, completion behavior, completion styling, and future browser verification.

Review activity is observable in two layers. Worker-internal reviewer subagents launched and terminated, and the plan artifacts record Reviewer B cycle 1 returning AR-01 through AR-03 for off-by-one browser verification wording, missing bug recovery, and missing evidence artifacts; after revisions, the final `adversarial-review.md` records fresh review cycle 2 approved with no findings. The harness Protocol 1.4.2 lifecycle, however, records only a Reviewer B launch and handoff with `exit_code=65`, plus worker-internal subagent events, so the harness lifecycle remains tainted rather than accepted.

The correction cycle strengthened the plan by making the future browser proof a five-click sequence, adding durable bug-register instructions for failed UI evidence, and adding context, validation, and analysis reports. Observable fix-cycle count is not recorded as a formal lifecycle metric, so the reproducible value is `not recorded`.

Final evidence is mixed: worker exit code 0, tagged validation passed, structural validation passed, process audit passed, telemetry was usable, and the HTML/HTM audit found zero forbidden artifacts. The benchmark is still tainted because reviewer lifecycle validation failed and the blinded oracle rejected the run; no access-audit escape was observed in the archived reports.
