# Batch overview

Total revisions benchmarked: 1.
Completed successfully: 0.
Tainted: 1.
Lacked usable telemetry: 1.

Only the run-local harness files were observable in the permitted archive tree. The harness summary listed one revision, `current`, with worker exit code `127`; no per-revision result archive was present under `/tmp/ai-skills-capsules/20260811T111202Z-current-fresh2/analysis/results`, so artifact, lifecycle, validation, telemetry, and plan counts are unavailable for that revision.

## Revision comparison

| Revision | Worker exit status | Validation result | HTML/HTM artifact audit | Session UUID | Telemetry records | Token total | Accepted/tainted status | Taint reasons |
|---|---:|---|---|---|---:|---|---|---|
| current | 127 | unavailable: no revision archive/evaluation.md observable | unavailable: no revision archive observable; 0 HTML/HTM artifacts observed in a per-revision archive | unavailable | 0 | unavailable: no telemetry records observable | tainted | worker exited non-zero; missing per-revision result archive in permitted results tree; missing evaluation.md; missing selected plan directory; missing telemetry; missing validation/analysis evidence; missing exact tagged `planning/` skill archive; Protocol 1.4.2 lifecycle, independence, handoff/termination, reviewer ownership/closures, verification passes, unresolved limits, and final fresh-review approval evidence unavailable |

# Deliverable inventory by revision

| Revision | Benchmark status | Selected plan directory | Integrity warnings | Goals | Work-unit inventory items | UI story/run-cache items | Testing companions | Review report files | Bug-register files | Context snapshots | Validation/analysis reports | Total files in selected plan directory | Total files in result archive | Telemetry records | Total usage tokens |
|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| current | failed: worker exit code 127 | unavailable: `evaluation.md` absent from observable revision archive | no per-revision result archive present; selected plan directory cannot be identified; extra candidate plan directories cannot be audited; exact tagged `planning/` skill archive missing/unobservable | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | 0 | unavailable |

Counting rule: counts include only files and records observable inside that revision's result archive under the permitted results tree. The selected plan directory must be the directory named by `evaluation.md`; because no `evaluation.md` or per-revision archive was observable, plan-directory counts are `0` and the selected directory is unavailable. Work units are counted only from ID rows such as `WU-01` or `W01` in a work-unit inventory, not from filenames. Review report files and bug-register files are counted as files, separately from review findings or bug entries. Ambiguous lifecycle categories such as review rounds and fix cycles are reported as `not recorded`.

# Token usage progression

| Revision | Previous revision | Previous tokens | Current tokens | Delta | Percentage change | Result | Evidence interpretation |
|---|---|---:|---:|---:|---:|---|---|
| current | none | unavailable | unavailable | unavailable | unavailable | comparison unavailable | first revision and no usable telemetry records were observable |

Overall interpretation: no token expansion or decrease can be measured for this batch. The single observed revision lacks usable telemetry, and no inventory or journey evidence from a per-revision archive is available to connect usage changes to planning-work expansion or reduction.

# Developer journey by revision

## current

- Approach: not recorded. The permitted archive tree contains no worker JSONL event sequence or selected plan artifacts for this revision.
- Review rounds and correction/fix cycles: not recorded. Protocol 1.4.2 reviewer sessions, review mode, verification passes, finding ownership/closures, handoff and termination events, independence status, unresolved limits, and final fresh-review approval are unavailable.
- Changes after review: not recorded. No review, analysis, validation, bug-register, context, or progress artifacts were observable in the revision archive.
- Final validation and constraints: the harness recorded worker exit code `127`, so the benchmark did not complete successfully. The revision is tainted because the per-revision result archive, telemetry, validation evidence, selected plan directory, and exact tagged `planning/` skill archive are missing from the permitted results tree.
