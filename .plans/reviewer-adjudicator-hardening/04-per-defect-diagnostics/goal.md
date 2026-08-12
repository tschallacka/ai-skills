# Goal: Per-defect explainable classification and report

## Current state and prior-goal handoffs

§ 2.1
The oracle report exposes only aggregate counts; the exact failed predicate and candidate findings are not surfaced. This hid RC-04: in the fresh run SD-01 had zero candidate findings because the reviewer never wrote a plan-description-scoped finding (confirmed via transcripts).

## Outcome and definition of done

§ 3.1
Make the grader output per-defect detail so analyst and future runs can see which predicate failed and which findings were considered. DoD: grader emits per_defect entries (defect_id, classification true_positive/partial/unresolved/false_positive, candidate finding ids, list of failed predicates) while preserving aggregate fields and public redaction; a deterministic post-run report is produced and archived; a defect with zero candidate findings (e.g. SD-01 in the fresh run) is attributable.

## Why this goal is needed

§ 4.1
Per-defect explainable classification turns an opaque 0/3 into an attributable report and directly implements analysis recommendation 1.

## Scope

§ 5.1
In: per-defect classification, public projection into protocol-metadata/telemetry, deterministic reproducible post-run report. Out: no change to acceptance thresholds or fail-closed semantics.

## Affected files, systems, data, and interfaces

§ 6.1
benchmark/planning/grade-blinded-run.sh report schema and private rows; setup-benchmark.sh protocol-metadata/telemetry aliases; benchmark comparison and analysis notes.

## Dependencies and handoffs

§ 7.1
W09 classification consumes the W05 matcher; W10 exposes the public projection; W11 proves reproducibility. Goal 05 W12 fixtures assert the per-defect detail.

## Implementation approach, risks, and edge cases

§ 8.1
The public report must never include expected_signal, required_correction, or seed IDs; keep those in private rows only. Classification names the exact failed predicate so an analyst can distinguish reviewer-observation from grader rejection.

## Owned work units

§ 9.1
 — Emit per-defect classification into private rows and a sanitized public projection (ordinal, public finding ids, failed predicates) with a partial category; never leak seed IDs.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | This goal owns W11 (verification: reproducible post-run report). |

§ 9.2
`W10` — Surface a public projection of per_defect diagnostics into protocol-metadata and telemetry aliases without leaking expected_signal, required_correction, or seed IDs.

§ 9.3
`W11` — Produce and archive a deterministic post-run report showing each defect, the candidate findings considered, and the exact failed predicate; verify it is reproducible across two invocations.

§ 9.4
`W09` — Emit per_defect entries with defect_id, classification (true_positive/partial/unresolved/false_positive), candidate finding ids, and the list of failed predicates, while keeping aggregate fields and public redaction.

## Goal-size exception

§ 11.1
<required only when this goal has one permitted work unit>
