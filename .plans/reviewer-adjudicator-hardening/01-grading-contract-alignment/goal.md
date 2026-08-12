# Goal: Grading contract alignment (path/location matching)

## Current state and prior-goal handoffs

§ 2.1
grade-blinded-run.sh filters every finding by exact candidate_path == defect path and a literal location match before any semantic check. Fresh AR-02/AR-03 used multi-file ; separated paths and prose locations; iterative AR-01 used line-based locations. All were filtered to false_positive before signal/correction, producing 0/3 (verified replay in .plans/reviewer-optimization-1-4-2/analysis-deep-dive.md).

## Outcome and definition of done

§ 3.1
Make path and location matching tolerant of natural reviewer output so schema-valid consolidated or prose findings can be graded. DoD: grade-blinded-run.sh accepts a defect file named in any ; separated path segment or resolvable from location/evidence; location normalization handles section/sec/3.1/line citations; a documented reviewer-evidence contract makes schema-valid equal gradeable; existing true-positive semantics for exact single-file/S-location findings are preserved.

## Why this goal is needed

§ 4.1
Tolerant matching is the primary fix for RC-01 and RC-02 and converts the frozen runs to their honest scores (iterative 3/3, fresh 2/3) without touching archived evidence.

## Scope

§ 5.1
In: grader path/location matching plus the reviewer prompt and schema contract note. Out: no reviewer observation-behavior change, no archived-archive edits.

## Affected files, systems, data, and interfaces

§ 6.1
benchmark/planning/grade-blinded-run.sh candidate_path and location_matches plus setup-benchmark.sh reviewer prompt generator (771-790) and approval schema validator (495-570).

## Dependencies and handoffs

§ 7.1
No prerequisite. Hands location normalization to W02 and the contract note to W03; goal 05 fixtures W12 and frozen replay W14 verify this goal's behavior downstream.

## Implementation approach, risks, and edge cases

§ 8.1
Keep the gate bounded: a finding must still reference the defect file through a path segment or a resolvable location, so findings about unrelated files never count. Preserve exact single-file/S-location true-positive semantics.

## Owned work units

§ 9.1
`W01` — Accept a defect file if it appears as any ; separated path segment or is resolvable from location or evidence; preserve exact single-file equality.

## Testing requirement

| Test required | Rationale |
|---|---|
| no | Path/location matching changes are behaviorally verified by goal 05 fixtures and frozen replay (W12, W14) which downstream-require these units; no test unit is owned by this goal. |

§ 9.2
`W02` — Normalize section markers (section/sec/3.1/#/:) and accept line- or prose-location citations that reference the defect file and section; keep exact S-location equality passing.

§ 9.3
`W03` — Document the reviewer-evidence contract (single-file path recommended when a defect is in one file, file section location recommended, prose accepted, consolidation valid) in the generated reviewer prompt and schema validator comments so schema-valid equals gradeable.

## Goal-size exception

§ 11.1
<required only when this goal has one permitted work unit>
