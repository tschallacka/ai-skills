# Batch overview

Total revisions benchmarked: 1.

Completed successfully: 0.

Tainted revisions: 1.

Revisions lacking usable telemetry: 1.

Evidence scope: this report used the observable harness files in this run's results directory and the current run archive area under `/tmp/ai-skills-capsules/20260811T105535Z-current-fresh/analysis/results`. The harness row points at a result directory outside the permitted archive scope, so that directory was not inspected. No per-revision result archive for `current` was observable under the allowed archive tree.

## Deliverable inventory by revision

| Revision | Benchmark status | Worker exit status | Validation result | HTML/HTM artifact audit | Session UUID | Protocol/cohort | Reviewer protocol evidence | Goals | Work-unit inventory items | UI story/run-cache items | Testing companions | Review report files | Bug-register files | Context snapshots | Validation/analysis reports | Selected plan directory | Extra candidate plan dirs | Total files in selected plan dir | Total files in revision result archive | Telemetry records | Total usage tokens | Accepted/tainted | Taint reasons |
|---|---:|---:|---|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---|---|---:|---:|---:|---|---|---|
| current | failed | 1 | unavailable | unavailable; no `.html`/`.htm` artifacts observable in a per-revision archive | unavailable | unavailable; no `protocol_id` record observable | unavailable; no reviewer lifecycle, reviewer sessions, cycle, verification pass, handoff, termination, independence, owner/closure, unresolved-limit, or final fresh-review approval evidence observable | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | unavailable; `evaluation.md` not observable | none observable | 0 | 0 | 0 | unavailable | tainted | worker exited non-zero; per-revision result archive missing from allowed archive tree; no usable telemetry; no selected plan directory from `evaluation.md`; no validation artifact; exact tagged `planning/` skill archive missing; Protocol 1.4.2 lifecycle and independence evidence unavailable |

Counting rule: counts are based only on files and records observable inside the revision's result archive under the allowed results tree. The selected plan directory must be the directory named by `evaluation.md`; because no `evaluation.md` or revision archive was observable for `current`, plan counts are reported as zero observable files and the selection is unavailable. Work units are counted from ID rows such as `WU-01` or `W01` in the work-unit inventory, not from filenames. Review report files and bug-register files are counted as files, separately from review findings or bug entries. Review rounds, fix cycles, and ambiguous lifecycle counts are reported as `not recorded` unless the archive contains explicit records.

## Token usage progression

No token progression can be computed for this batch.

| Revision | Previous revision | Previous tokens | Current tokens | Delta | Percent change | Direction | Assessment |
|---|---|---:|---:|---:|---:|---|---|
| current | none | unavailable | unavailable | comparison unavailable | comparison unavailable | comparison unavailable | First and only revision, with no usable telemetry records in the allowed archive tree. |

Overall interpretation: the batch provides no usable token totals, so there is no observable token expansion or decrease across revisions. Any comparison would require telemetry from a per-revision archive, which is absent here.

## Developer journey by revision

### current

The harness attempted the `current` revision in fresh-review mode, but the worker exited with status `1`. No worker JSONL event sequence, selected plan directory, `progress.md`, review report, analysis report, validation report, bug register, context snapshot, or run-cache artifact was observable in the allowed per-revision archive area. Review rounds: not recorded. Correction/fix cycles: not recorded. Final validation, handoff and termination evidence, reviewer independence, and fresh-review approval are unavailable, so the revision is tainted rather than accepted.
