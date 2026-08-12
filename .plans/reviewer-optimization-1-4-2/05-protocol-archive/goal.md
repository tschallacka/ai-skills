# Goal: Transition benchmark cohorts and archival metadata

## Current state and prior-goal handoffs

§ 2.1
The benchmark currently stores timestamped run directories and copies tagged planning skills, but it has no explicit 1.4.2 protocol/cohort metadata or atomic publish boundary.

## Outcome and definition of done

§ 3.1
Start protocol 1.4.2 with legacy-cohort separation, self-describing atomic result archives, protocol/reviewer/access metadata, and merged analyzer reporting.

## Why this goal is needed

§ 4.1
Legacy results must remain scientifically frozen while new results become self-describing and analyzable without pretending cross-cohort comparisons are controlled.

## Scope

§ 5.1
Include run metadata, staged atomic result publication, analyzer cohort merge/report rules, operator documentation, and v27 manifest/map ownership. Exclude rerunning or modifying legacy archives.

## Affected files, systems, data, and interfaces

§ 6.1
Change benchmark/planning/run-benchmark.sh, setup-benchmark.sh, analyzer-prompt.md, benchmark-test.md, README.md, planning/V27-PACKAGE-MANIFEST.txt, and planning/V27-PACKAGE-MAP.tsv.

## Dependencies and handoffs

§ 7.1
Consume lifecycle and capsule metadata from Goals 2–4. Hand off a stable archive schema and protocol identifier to safeguards and pilot analysis.

## Implementation approach, risks, and edge cases

§ 8.1
Stage artifacts in a run-specific directory, validate telemetry and required outputs before publish, include exact tagged skill provenance, separate legacy/new summaries, and label ambiguous counts not recorded.

## Owned work units

§ 9.1
`W21` — Start protocol 1.4.2 as a distinct cohort, preserve legacy results unchanged, and record protocol, reviewer mode, and access-control mode in run metadata.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The goal changes executable benchmark or planning behavior and owns focused tests or verification units in the inventory. |

§ 9.2
`W22` — Stage each run under a timestamp/name directory, include skill/version and protocol metadata, and publish atomically only after artifact and telemetry checks pass.

§ 9.3
`W23` — Merge legacy and 1.4.2 results into one report with separate cohort summaries, protocol labels, contextual cross-cohort comparisons, and authoritative within-cohort metrics.

§ 9.4
`W24` — Document frozen legacy treatment, 1.4.2 start boundary, metadata requirements, unchanged historical layout, and analyzer comparison rules.

§ 9.5
`W25` — Document the user-facing setup command, hidden protocol metadata, capsule lifecycle, pilot command, and result archive locations without exposing implementation differences.

§ 9.6
`W26` — Extend the finite manifest for the v27 replacement package to include the changed contract, helpers, benchmark/oracle records, fixtures, and runner evidence.

§ 9.7
`W27` — Map every new or changed package file to its install destination and preserve the explicit six-column ownership boundary.

§ 9.8
`W50` — Stage evaluation and copied artifacts under a private run directory, validate all preconditions, atomically rename on success, and clean up on failure/collision/interruption.

§ 9.9
`W51` — Test worker/reviewer/analyzer/telemetry failures, collision behavior, rollback cleanup, atomic visibility, and exact tagged-skill provenance.

## Goal-size exception

§ 11.1
Not applicable: this goal owns multiple work units and does not use the single-unit exception.
