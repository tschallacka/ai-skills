# Goal: Reviewer authority and provenance

## Current state and prior-goal handoffs

§ 2.1
The current state calculation aggregates every approval.json and reported conflict when Reviewer A was true while Reviewer B was false. The archive also does not expose enough linked hashes to explain which seeded copy and reviewer artifact were graded.

## Outcome and definition of done

§ 3.1
Make Reviewer B the sole final approval authority, reject unauthorized Reviewer A approvals, and preserve exact reviewer/capsule/seed provenance in the archived decision state.

## Why this goal is needed

§ 4.1
The protocol explicitly assigns final approval to Reviewer B. Mixing worker, Reviewer A, and Reviewer B artifacts creates false conflicts and weakens attribution. Immutable provenance is needed to explain every oracle decision.

## Scope

§ 5.1
In scope are final approval selection, Reviewer A prohibition, duplicate and missing approval states, archive metadata, seed snapshot identity, capsule identity, transcript identity, and analyzer interpretation. Out of scope are changes to thresholds or retrospective rewriting of old archives.

## Affected files, systems, data, and interfaces

§ 6.1
W05, W06, W13, and W15 change setup-benchmark.sh lifecycle, approval validation, state, and publication logic. W07 changes benchmark/planning/analyzer-prompt.md so reports distinguish authority, schema, transformation, and provenance failures.

## Dependencies and handoffs

§ 7.1
W05 consumes the envelope and selects Reviewer B candidates by role only. W13 validates approval schema and finding shapes; W15 exclusively binds the selected candidate to the exact B session/capsule before W06 publishes identity and hashes. W07 consumes those fields for analysis. Goal 03 validates that the state is deterministic under approval permutations.

## Implementation approach, risks, and edge cases

§ 8.1
Select Reviewer B candidates by lifecycle role, validate approval.json shape before use, and delegate every exact session/capsule/mode/freshness equality check exclusively to W15. Treat Reviewer A as handoff-only, fail closed on unauthorized overall approval, and report distinct reasons for missing, duplicate, conflicting, rejected, or schema-invalid evidence. Publish hashes for the source plan, defective target, target snapshot, approval, transcript, lifecycle handoff, and selected capsule.

## Owned work units

§ 9.1
`W05` — Make Reviewer B the only final approval authority.

§ 9.2
`W06` — Publish selected reviewer, capsule, seed, approval, transcript, and plan provenance.

§ 9.3
`W07` — Require analysis to distinguish lifecycle, schema, provenance, and semantic outcomes.

§ 9.4
`W12` — Test authority selection and provenance through the goal-local regression.

§ 9.5
`W13` — Validate Reviewer B approval schema and finding shapes before serialization.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | Approval selection and provenance publication are runtime state contracts and require lifecycle and archive assertions. |

§ 9.6
`W15` — Bind the selected approval to the exact Reviewer B lifecycle session and capsule; reject wrong-session, wrong-mode, stale, duplicate, and mismatched artifacts before terminal evidence.

## Goal-size exception

§ 11.1
No exception: the goal owns six authority/provenance units and two goal-local regression/validation tests.
