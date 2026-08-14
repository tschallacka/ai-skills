# Goal: Record benchmark evidence and final audit

## Current state and prior-goal handoffs

§ 2.1
The plan has implementation, readiness, and UI verification work units. This goal records the benchmark evidence created by the planning-only proof itself and audits the selected plan directory before final handoff.

## Outcome and definition of done

§ 3.1
Record revision, timestamps, elapsed time, worker result, validation/review status, artifact audit, filesystem-boundary audit, thread ID, and token-usage availability for this planning-only proof.

## Why this goal is needed

§ 4.1
The benchmark task requires a durable analysis report with execution metadata and evidence, so report creation and final audit need explicit ownership.

## Scope

§ 5.1
Included: analysis-report.md, final validation.md capture, non-empty artifact audit, forbidden HTML/HTM audit, review outcome, session ID, and token-usage availability.

§ 5.2
Excluded: repairing worker telemetry outside the runner, inspecting parent directories, opening HTML, or auditing unrelated repository files.

## Affected files, systems, data, and interfaces

§ 6.1
Planning evidence files: analysis-report.md and validation.md in the selected plan directory. Verification target: isolated benchmark workspace artifact listing and forbidden HTML/HTM search.

## Dependencies and handoffs

§ 7.1
Depends on the plan artifacts and final review. The final response can rely on W07 and W08 once the report and audit are complete.

## Implementation approach, risks, and edge cases

§ 8.1
Use actual command output for validation and artifact audit. Record token usage as unavailable unless UUID-matched telemetry is exposed by the runner.

§ 8.2
If the final audit finds a missing mandatory artifact or forbidden HTML/HTM file, create or correct the substantive planning artifact before completion; do not waive the result.

## Owned work units

§ 9.1
`W07` — Write the analysis report with exact start/end timestamps, elapsed time, worker result, validation results, review result, artifact/process audit, thread ID, and token-usage availability.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The goal owns final benchmark evidence and a verification audit work unit. |

§ 9.2
`W08` — Audit only the benchmark workspace for expected plan output, mandatory non-empty artifacts, and absence of forbidden HTML/HTM files before completion.

## Goal-size exception

§ 11.1
Not applicable; this goal owns two work units, within the 2-10 work-unit limit.
