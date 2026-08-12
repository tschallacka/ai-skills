# Goal: Define the 1.4.2 reviewer and protocol contract

## Current state and prior-goal handoffs

§ 2.1
HEAD already contains an early 1.4.2 reviewer profile generator, REVIEWER.md, marked source sections, and v27 context/package machinery. The remaining work is to rebaseline those artifacts against the exact benchmark-tagged source and close the contract/test gaps required by the brainstorm.

## Outcome and definition of done

§ 3.1
Make SKILL.md authoritative for iterative review, reviewer-profile generation, and bounded lifecycle fields, with a reproducible generated REVIEWER.md.

## Why this goal is needed

§ 4.1
All later runner, capsule, context, and analyzer changes depend on one explicit contract for reviewer ownership, independence, limits, and metadata.

## Scope

§ 5.1
Include only the planning skill contract, profile generator/output, and focused generation tests. Exclude benchmark runtime behavior, which belongs to later goals.

## Affected files, systems, data, and interfaces

§ 6.1
Change planning/SKILL.md, planning/scripts/generate-reviewer.sh, planning/REVIEWER.md, and planning/tests/test-plan-commands.sh.

## Dependencies and handoffs

§ 7.1
No prerequisite implementation is required. Hand off stable field names, section markers, version metadata, and rejection rules to Goals 2–6.

## Implementation approach, risks, and edge cases

§ 8.1
Mark reviewer-source sections explicitly, generate only the allowlisted content, fail closed on missing/empty sections, and test hash drift. Treat profile output as generated and never hand-edit it.

## Owned work units

§ 9.1
`W01` — Define authoritative iterative-review mode, reviewer lifecycle fields, bounded pass/cycle limits, contamination rules, access-control boundary, protocol metadata, telemetry schema, and legacy-cohort transition.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The goal changes executable benchmark or planning behavior and owns focused tests or verification units in the inventory. |

§ 9.2
`W02` — Enforce REVIEWER_SECTION allowlisting, required-section validation, source hashing, version 1.4.2 metadata, and non-hand-edited output generation.

§ 9.3
`W03` — Regenerate the reviewer profile from the tagged source sections and record the source hash and version without hand edits.

§ 9.4
`W04` — Test marked-section extraction, missing/empty section rejection, hash/version metadata, and generated-profile drift detection.

§ 9.5
`W05` — Run generator and the focused planning contract tests against a clean temporary copy and prove the committed profile matches generated output.

## Goal-size exception

§ 11.1
Not applicable: this goal owns multiple work units and does not use the single-unit exception.
