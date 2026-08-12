# Goal: Lossless reviewer finding contract

## Current state and prior-goal handoffs

§ 2.1
The current direct oracle accepts a complete consolidated AR-01 fixture, but the live benchmark path dropped reviewer fields before grading. This goal starts from that confirmed adapter and schema gap and owns the canonical finding contract.

## Outcome and definition of done

§ 3.1
Define and enforce one complete machine-readable finding envelope from Reviewer B approval through terminal oracle evidence, preserving consolidated findings and rejecting incomplete evidence.

## Why this goal is needed

§ 4.1
A reviewer can detect a defect without receiving credit if evidence transport is lossy. A single canonical envelope prevents detection quality from being confused with serialization failure and makes malformed evidence visible.

## Scope

§ 5.1
In scope are the setup adapter, blinded grader envelope rules, Reviewer B prompt requirements, and worker-facing finding contract. Out of scope are approval authority selection, archive provenance, seeded fixture semantics, and application behavior.

## Affected files, systems, data, and interfaces

§ 6.1
W01 changes the approval-to-oracle serialization in benchmark/planning/setup-benchmark.sh. W02 changes finding validation and matching in benchmark/planning/grade-blinded-run.sh. W03 changes the reviewer prompt construction in setup-benchmark.sh. W04 changes the worker-facing protocol in benchmark/planning/worker-prompt.md.

## Dependencies and handoffs

§ 7.1
W01 defines the fields transported to W02. W02 defines the schema that W03 and W04 must require. Goal 02 consumes the stable evidence envelope, and Goal 03 proves the full path.

## Implementation approach, risks, and edge cases

§ 8.1
Use one explicit envelope with finding_id, path, location, summary, observed_contradiction, impact, evidence, required_correction, and independent. Preserve consolidated findings as one object. Reject incomplete objects with a machine-readable schema result containing a per-finding reason and aggregate malformed count; never let a classification label substitute for semantic evidence.

## Owned work units

§ 9.1
`W01` — Preserve the complete approved finding envelope in oracle-terminal-evidence.json.

§ 9.2
`W02` — Validate the envelope and classify malformed evidence without losing consolidated matching.

§ 9.3
`W03` — Require the complete finding schema in the Reviewer B prompt.

§ 9.4
`W04` — Require the same machine-readable contract in the worker-facing protocol.

§ 9.5
`W11` — Test the exact fields through the goal-local regression.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The adapter, schema, prompt, and worker contract are executable and must be proven by direct and integration tests. |

§ 10.1
The goal-local regression covers the serialized envelope and semantic grading path; Goal 02 may consume only complete, schema-valid finding objects.

## Goal-size exception

§ 11.1
No exception: the goal owns four contract units and one goal-local regression test.
