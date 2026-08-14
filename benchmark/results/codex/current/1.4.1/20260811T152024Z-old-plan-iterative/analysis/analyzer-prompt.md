Analyze the completed planning benchmark batch `20260811T152024Z-old-plan-iterative`.

You may inspect only:

- `/home/mdibbets/git/ai-skills/benchmark/results/20260811T152024Z-old-plan-iterative/analysis/benchmark-test.md`
- `/home/mdibbets/git/ai-skills/benchmark/results/20260811T152024Z-old-plan-iterative/analysis/harness-summary.tsv`
- result archives under `/tmp/ai-skills-capsules/20260811T152024Z-old-plan-iterative/analysis/results` for this run

Write the comparison report to:

```text
/tmp/ai-skills-capsules/20260811T152024Z-old-plan-iterative/analysis/results/comparison.md
```

Include one row per revision and separate worker exit status, validation
result, HTML/HTM artifact audit, session UUID, telemetry records, token total
or unavailable status, accepted/tainted status, and taint reasons.

Start the report with a `Batch overview` section stating the total number of
revisions benchmarked, how many completed successfully, how many were tainted,
and how many lacked usable telemetry. Then add a `Deliverable inventory by
revision` table with one row per revision. Count only observable files and
records in that revision's result archive; include at least:

- revision and benchmark status;
- goals, work-unit inventory items, UI story/run-cache items, testing
  companions, review reports, bug-register items, context snapshots, and
  validation/analysis reports;
- total files in the plan directory and total files in the result archive;
- telemetry record count and total usage tokens.

Use the selected plan directory named by `evaluation.md` for all plan counts.
If extra candidate plan directories are present, report them as an integrity
warning and exclude them from the counts. Count work units by counting ID rows
in the work-unit inventory (`WU-01`, `W01`, etc.), never by counting files
whose names contain `work-unit`. Count review reports and bug-register files
separately from review findings or bug entries, and state the counting rule.
The revision result must also contain the exact tagged `planning/` skill; report
it as missing if this older run predates that archive requirement.

Use exact counts when the artifacts support them. For review rounds, fix
cycles, or any category whose boundaries are ambiguous, use `not recorded`
rather than inventing precision. Explain the counting rule in one short note
below the table so the counts can be reproduced from the archived artifacts.

After that table, add a `Token usage progression` section. Sort revisions by
semantic version when possible, otherwise use the order in the harness summary.
For every revision after the first usable-token revision, report the previous
revision, both token totals, the absolute delta, and percentage change using
`((current - previous) / previous) * 100`. Label the result as an increase or
decrease. Also state whether the change is only an observed usage difference or
appears plausibly related to expansion/reduction in planning work, based on the
inventory and journey evidence; do not claim causation from token totals alone.
If either side lacks usable telemetry, say `comparison unavailable` and explain
why. End this section with a short overall interpretation of token expansion
or decrease across the batch.

After the comparison table, include a short `Developer journey by revision`
section with one subsection per revision. Each subsection should be roughly
2–5 sentences or a compact four-bullet summary covering:

- how the worker approached the planning task;
- the observable number of review rounds and correction/fix cycles;
- what changed or was strengthened after review;
- the final validation, evidence, and notable constraint or failure.

Make the summaries engaging and specific, but describe only observable actions
and outcomes—not private chain-of-thought or speculative inner reasoning. Use
the worker JSONL event sequence and the plan artifacts (`progress.md`, review,
analysis, validation, bug, and context reports) as evidence. If a review or
fix count cannot be established, say `not recorded` rather than guessing.
Keep each revision’s journey distinct and mention meaningful differences in
how the revisions progressed.

Do not repair worker artifacts and do not fill missing telemetry from other
sessions.

Protocol 1.4.2 reports must separate review mode, reviewer sessions, cycles,
verification passes, finding owners/closures, handoff and termination events,
independence status, unresolved limits, and final fresh-review approval.
Missing lifecycle or independence evidence is unavailable/tainted; never infer
review rounds from token totals or prose.

Every current-protocol comparison row and `evaluation.md` state section must
preserve this machine-readable approval contract without collapsing it into
worker exit status or oracle status:

```json
{
  "review_completed": true,
  "plan_approved": true,
  "oracle_completed": true,
  "adoptable": true,
  "semantic_true_positive_rate": 1.0,
  "independent_catch_rate": 1.0,
  "seeded_denominator": 3,
  "fail_closed_reasons": [],
  "approval_conflict": false
}
```

`adoptable` is true only when all four booleans are true, the denominator is
positive, both configured semantic and independent-catch thresholds exist and
are met, no ambiguity or taint is present, and `fail_closed_reasons` is empty.
False approval remains gradeable detection evidence but requires
`PLAN_NOT_APPROVED` and is non-adoptable. Missing approval also reports
`APPROVAL_MISSING`; conflicting approval reports `APPROVAL_CONFLICT`.
Incomplete review/oracle, ambiguous oracle output, tainted runs, missing
denominators or thresholds, and below-threshold semantic or independent rates
must use the corresponding enum reason. Reasons are deduplicated and sorted
lexically; retain all applicable reasons. Mechanical exact-ID diagnostics are
reported separately from semantic rates. Do not rewrite historical archives or
infer a state field from prose, exit codes, or a missing artifact.

Worker-internal reviewer subagents may appear in `reviewer-lifecycle.jsonl`
with `actor=worker-subagent` and `protocol_role=worker-internal`. Report those
sessions separately as observed worker activity; do not count them as harness
Reviewer A/B protocol sessions or taint the harness lifecycle merely because
they are not assigned protocol ownership fields.

The analyzer may read only its run instructions, harness summary, current run
archive, and its own workspace. It must not read the source checkout,
installed skills, parent paths, or prior run archives. Preserve access-audit
evidence and mark any attempted escape as tainted.

Treat archives with `protocol_id=reviewer-optimization-1.4.2` as a distinct
1.4.2 cohort. Keep legacy archives frozen and report them separately; any
cross-cohort comparison is contextual only. Within-cohort metrics and
protocol-labelled lifecycle evidence are authoritative.
