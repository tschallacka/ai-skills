# Batch overview

Run `20260810T202618Z-pilot-smoke` benchmarked 1 revision: `1.4.1`.

- Completed successfully: 0
- Tainted: 1
- Lacked usable telemetry: 1

The allowed result tree does not contain a per-revision archive for `1.4.1`; only `results/harness-summary.tsv` and an empty `results/analysis/` directory are observable. The harness summary lists `1.4.1` with worker exit/status value `1`, but the referenced archive path is outside the allowed `/tmp/.../analysis/results` archive tree and was not inspected. This report therefore treats the revision as tainted and reports archive-dependent evidence as unavailable.

| Revision | Cohort / protocol evidence | Worker exit status | Validation result | HTML/HTM artifact audit | Session UUID | Telemetry records | Token total | Accepted / tainted | Taint reasons |
|---|---|---:|---|---|---|---:|---|---|---|
| `1.4.1` | protocol unavailable; no `evaluation.md` archive evidence | `1` | unavailable: revision archive missing | unavailable: revision archive missing | unavailable | unavailable | unavailable | tainted | Worker exit/status is non-zero in harness summary; no allowed per-revision result archive; no `evaluation.md`; no UUID-matched telemetry; no selected plan directory; no validation evidence; no HTML/HTM audit evidence; exact tagged `planning/` skill archive missing; Protocol 1.4.2 lifecycle/independence evidence unavailable. |

# Deliverable inventory by revision

| Revision | Benchmark status | Selected plan directory | Integrity warnings | Goals | Work-unit inventory items | UI story/run-cache items | Testing companions | Review reports | Bug-register items | Context snapshots | Validation/analysis reports | Plan files | Result archive files | Telemetry records | Total usage tokens |
|---|---|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| `1.4.1` | tainted; harness exit/status `1` | unavailable: no `evaluation.md` in allowed archive | Missing per-revision archive under `/tmp/ai-skills-capsules/20260810T202618Z-pilot-smoke/analysis/results`; referenced `/home/.../1.4.1` archive was not inspected under the analyzer access limits. | unavailable | unavailable | unavailable | unavailable | unavailable | unavailable | unavailable | unavailable | unavailable | 0 | unavailable | unavailable |

Counting rule: all counts are limited to observable files and records in the revision's own result archive. The selected plan directory must be the one named by `evaluation.md`; because no revision archive or `evaluation.md` is present, no plan counts can be made. Work units would be counted from ID rows such as `WU-01` or `W01` in the work-unit inventory, not from filenames. Review report files and bug-register files are counted separately from review findings or bug entries. Ambiguous review rounds, fix cycles, verification passes, and finding closure boundaries are reported as `not recorded`.

# Token usage progression

| Revision | Previous usable-token revision | Previous tokens | Current tokens | Delta | Change | Interpretation |
|---|---|---:|---:|---:|---|---|
| `1.4.1` | none | unavailable | unavailable | unavailable | comparison unavailable | No UUID-matched telemetry is observable in the allowed per-revision archive because that archive is missing. |

Overall interpretation: no token expansion or decrease can be established for this batch. There is only one revision in the harness summary, and it lacks usable telemetry in the allowed artifacts, so any token comparison would be unsupported.

# Developer journey by revision

## `1.4.1`

- Approach: not recorded. The worker JSONL sequence and selected plan artifacts are unavailable because the allowed result archive for this revision is missing.
- Review rounds and correction/fix cycles: not recorded. Protocol 1.4.2 requires separate review mode, reviewer sessions, cycles, verification passes, owner/closure evidence, handoff and termination events, independence status, unresolved limits, and final fresh-review approval; none of that lifecycle evidence is observable here.
- Strengthening after review: not recorded. No review, bug-register, validation, analysis, progress, or context artifacts are available in the allowed archive tree.
- Final validation and constraints: final validation is unavailable. The notable failure is the absence of the per-revision archive under the permitted results directory, plus a non-zero worker exit/status value of `1` in the harness summary.

# Access audit

Inspected only the copied benchmark instructions and harness summaries in the analysis capsule plus the current run's `/tmp/.../analysis/results` tree. No source checkout, installed skill directory, parent path, prior run archive, or out-of-capsule result archive was inspected. The referenced `/home/mdibbets/git/ai-skills/benchmark/results/20260810T202618Z-pilot-smoke/1.4.1` archive named by the harness summary was excluded because it is outside the user-specified `/tmp/.../analysis/results` result-archive scope.
