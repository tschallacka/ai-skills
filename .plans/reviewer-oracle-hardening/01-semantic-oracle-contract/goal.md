# Goal: Semantic blinded-oracle contract

## Current state and prior-goal handoffs

The current grader compares reviewer `finding_id` values directly with hidden
seed IDs. This misclassified Reviewer B's `AR-01`, which described all three
seeded defects, as a false positive. The pilot's semantic shape is captured in
the deterministic W10 fixture; prior result archives remain immutable evidence
only and are never the test source of truth.

## Outcome and definition of done

Define and implement a private semantic defect manifest plus independent
adjudication contract. A finding can cover multiple defects, and the report
must distinguish semantic coverage from exact-ID coverage. Ambiguous coverage
is unresolved, not silently accepted.

## Why this goal is needed

Exact-ID grading cannot evaluate a reviewer who correctly consolidates related
defects. Semantic coverage is the primary correctness outcome of the oracle.

## Scope

In scope: seed manifest fields, encrypted private material, terminal evidence,
adjudication input/output, semantic coverage counts, and fixture tests.

Out of scope: reviewer access to hidden IDs, historical archive changes, and
one-finding-per-defect requirements.

## Affected files, systems, data, and interfaces

Coordinate the semantic changes across `seed-blinded-defects.sh`,
`grade-blinded-run.sh`, `review-oracle.sh`, and oracle contract tests; W01-W03
and W12 are the exclusive owners of their respective source and test scopes.

## Dependencies and handoffs

No prerequisite goal. Handoff to Goal 02 is the semantic report schema and
explicit distinction between exact-ID diagnostics and semantic coverage.

## Implementation approach, risks, and edge cases

Keep mutation deterministic. Record path, location, expected signal, required
correction, and severity in the encrypted manifest. Accept one-to-many mapping
only when the adjudicator evidence identifies each defect; test multiple defects
in one file and one consolidated AR finding.

## Owned work units

W01 owns manifest validation. W02 owns semantic grading and metrics. W03 owns
the redacted adjudication envelope. W12 owns manifest schema regression tests.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | Seed schema and semantic matching are correctness-critical and must be regression-tested. |
