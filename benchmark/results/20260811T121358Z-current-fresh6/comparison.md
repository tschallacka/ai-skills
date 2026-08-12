# Batch overview

Batch `20260811T121358Z-current-fresh6` benchmarked 1 revision in the `reviewer-optimization-1.4.2` fresh-review cohort. 1 revision completed with worker exit code 0, 0 revisions were accepted, 1 revision was tainted, and 0 revisions lacked usable telemetry.

The only benchmarked revision was `current`. Its worker artifact passed plan validation, structural validation, process audit, and HTML/HTM audit, but the benchmark result is tainted because the reviewer lifecycle failed and the blinded oracle rejected the result for missing terminal reviewer evidence.

## Revision comparison

| Revision | Worker exit | Validation | Structural validation | HTML/HTM artifact audit | Process audit | Session UUID | Telemetry records | Usage tokens | Tagged `planning/` skill | Benchmark status | Taint reasons |
|---|---:|---|---|---|---|---|---:|---:|---|---|---|
| `current` | 0 | pass | pass | pass, 0 HTML/HTM files | pass | `019ff0be-6ceb-7983-bbef-70e0fde5081a` | 1 | 4,390,875 | present, archived as `planning/` with 175 files; `evaluation.md` records tagged source `current` | tainted | `REVIEWER_LIFECYCLE_FAILED`; `BLINDED_ORACLE_FAILED`; oracle reason: target did not reach terminal reviewer evidence |

Protocol 1.4.2 evidence for `current`: review mode was `fresh-review`. `reviewer-lifecycle.jsonl` records three worker-internal subagent launches/terminations at worker cycle 0 with `actor=worker-subagent` and `protocol_role=worker-internal`; these are reported as worker activity, not harness Reviewer A/B sessions. The only protocol reviewer record is Reviewer B session `20260811T121358Z-current-fresh6-current-B-1786451375710133595`, with cycle 1 launch and handoff events, `verification_pass=1`, `independence=true` on handoff, and `exit_code=65`; a separate successful termination event is unavailable. Protocol Reviewer A, finding-owner assignments, finding-closure lifecycle, successful terminal approval, and final fresh-review approval are unavailable, so the lifecycle is unresolved/tainted. `approval.json` in the Reviewer B archive has `overall_plan_approval=false` and records approved findings `AR-02` and `AR-03` against the reviewed copy plus rejected finding `AR-01`.

Access/audit evidence: the worker analysis reports no unauthorized filesystem escape, no HTML creation or inspection, and no browser/server/driver execution. It also reports one blocked destructive delete attempt against this run's incomplete generated plan directory, followed by a bounded cleanup inside `BENCH_ROOT`; no source checkout, installed skill, parent path, or prior archive access was observed in the archived worker evidence.

## Deliverable inventory by revision

| Revision | Benchmark status | Selected plan directory | Integrity warnings | Goals | Work-unit inventory items | UI stories | UI run-cache items | Testing companions | Review report files | Review finding rows | Bug-register files | Bug entries | Context snapshots | Validation reports | Analysis reports | Plan files | Result archive files | Telemetry records | Usage tokens |
|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `current` | tainted | `basic-test-proof-current-20260811T121358Z-current-fresh6-isolated-plan` | none; exactly one candidate plan directory found | 2 | 7 | 2 | 2 | 7 | 1 | 1 | 1 | 0 | 1 | 1 | 1 | 29 | 258 | 1 | 4,390,875 |

Counting rule: the selected plan directory is the `Plan:` value in `evaluation.md`; extra candidate plan directories are any sibling directories with a `work-unit-inventory.md` and are excluded from counts. Work units are counted only as ID rows matching `W01`, `W02`, etc. in `work-unit-inventory.md`, not by filenames. Review report files are counted separately from `AR-` finding rows, and bug-register files are counted separately from `BUG-` entry rows. UI story count uses `US-` rows in `ui-user-stories.md`; run-cache count uses files under `ui-story-runs/`; testing companions are `*-testing.md` step files.

## Token usage progression

Sorted order falls back to the harness summary order because `current` is not a semantic version.

| Revision | Previous usable revision | Previous tokens | Current tokens | Delta | Change | Interpretation |
|---|---|---:|---:|---:|---|---|
| `current` | none | unavailable | 4,390,875 | unavailable | comparison unavailable | Baseline only; there is no prior usable-token revision in this batch. |

Overall interpretation: this batch provides no within-cohort token progression because it contains a single usable-token revision. The observed 4,390,875-token worker total is tied to a broad planning workflow with helper discovery, three worker-internal fresh review attempts, corrections, validation reruns, and final reporting, but no token increase or decrease can be calculated from this batch alone.

## Developer journey by revision

### `current`

- The worker started by recording session UUID `019ff0be-6ceb-7983-bbef-70e0fde5081a`, then read the benchmark instructions, task spec, tagged `planning/SKILL.md`, and the tagged UI validation reference from the allowed worker capsule as shown in `worker.jsonl`. It used the tagged planning helper scripts to generate a durable planning-only proof for `button-chain.html` without creating or testing any HTML.
- Observable worker-internal review activity went through three fresh reviewer subagent attempts according to `worker.jsonl`, `context-snapshot.md`, and `analysis-report.md`: first feedback covered pending review placeholders, UI rationale/goal placeholders, under-specified run cache, missing non-last-button guard coverage, and a case-mismatched plan manifest; second feedback covered remaining placeholders and misleading testing-companion headings; the third worker-internal review was recorded by the worker as approved. Harness protocol reviewer rounds are not equivalently established: the 1.4.2 lifecycle has only one Reviewer B protocol session and ends with `exit_code=65`.
- Corrections strengthened the plan by adding W07 for the non-last-button guard, adding US-02 and `ui-story-runs/US-02.md`, expanding US-01/US-02 run caches into per-click readiness checks, rewriting the actual uppercase plan path manifest, replacing placeholder progress text, and clarifying that testing companions describe future source/browser verification rather than executed tests.
- Final worker validation passed: `validation.md` records exit code 0 and `Plan validation passed: 7 work units across 2 goals.` The archive also shows zero HTML/HTM files and a passed process audit. The notable failure is outside the worker's local validator: protocol lifecycle evidence is incomplete and the blinded oracle rejected terminal reviewer evidence; the Reviewer B archive's `approval.json` records `overall_plan_approval=false`, so the revision is tainted despite complete-looking plan artifacts.
