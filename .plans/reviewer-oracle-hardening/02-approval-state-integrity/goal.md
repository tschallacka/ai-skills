# Goal: Approval-state integrity

## Current state and prior-goal handoffs

The pilot produced valid Reviewer B evidence with `overall_plan_approval=false`
but the harness still recorded the run as accepted because blinded-oracle mode
allowed negative approval. Review completion, defect detection, plan approval,
and adoption eligibility are currently conflated.

## Outcome and definition of done

Represent review completion and plan approval as separate states. A completed
review with unresolved findings can be graded for detection, but cannot be
accepted as an adoptable plan. Conflicting approval artifacts fail closed.

## Why this goal is needed

The pilot produced valid detection evidence but was labeled accepted despite
false overall approval. That can hide an unusable reviewer result.

## Scope

Coordinate `setup-benchmark.sh`, reviewer prompt/schema validation, protocol
metadata, evaluation output, and analyzer instructions; W04-W06 and W13-W16
are the exclusive owners of their respective source, generated, and test
scopes.

## Affected files, systems, data, and interfaces

The benchmark harness, generated reviewer contract, approval JSON schema,
protocol metadata, and evaluation report are affected. Historical reports are
read-only.

## Dependencies and handoffs

Depends on Goal 01's semantic report fields. Handoff to Goal 03 is an explicit
state machine and fail-closed adoption predicate.

## Implementation approach, risks, and edge cases

Validate approval schema and source location, canonicalize one reviewer-owned
approval artifact, and emit explicit `review_completed`, `plan_approved`,
`oracle_completed`, and `adoptable` states. Treat missing, conflicting, or
false approval as non-adoptable. Preserve false approval as useful detection
evidence rather than aborting before grading.

## Owned work units

W04 owns harness approval state. W05 owns the source reviewer contract. W06
owns analyzer/report instructions. W13 owns analyzer schema tests. W14 owns
the generated reviewer projection, W15 owns the worker prompt, and W16 owns
protocol metadata/evaluation state publication.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | Approval-state errors can incorrectly authorize adoption and require contract tests. |
