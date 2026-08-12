# Goal: Regression coverage and release gates

## Current state and prior-goal handoffs

The existing focused tests cover exact-ID grading and role enforcement but not
semantic consolidated findings or approval-state conflicts. Goals 01 and 02
will provide the new schemas and state fields.

## Outcome and definition of done

Build a regression suite and release-gate checks that reproduce the pilot failure,
prove the corrected semantic result, and prevent accepted status when approval
or required thresholds fail.

## Why this goal is needed

The pilot failure was only visible after comparing semantic review evidence with
the oracle output. Permanent fixtures are needed to keep that mismatch from
returning.

## Scope

In scope: unit/contract fixtures, the pinned pilot fixture,
public/private-boundary tests, end-to-end seeded run assertions, analyzer
report validation, plan validation, shell syntax, and resource-capped regression
execution. The end-to-end gate must exercise the real current worker, reviewer,
oracle, analyzer, and archive path. Out of scope: historical reruns,
performance claims, and unrelated planning behavior.

## Affected files, systems, data, and interfaces

Own benchmark contract tests, analyzer/report assertions, and current-protocol
release verification. No historical archive is modified.

## Dependencies and handoffs

Depends on Goals 01 and 02. The final handoff must include commands, expected
metrics, archive/report paths, and explicit remaining adoption gates.

## Implementation approach, risks, and edge cases

Run deterministic fixtures first, then a bounded current-protocol integration
check. Keep semantic scoring, approval state, and adoption thresholds separate;
any missing denominator or conflicting evidence remains fail-closed.

## Owned work units

W07 owns semantic oracle regression fixtures. W08 owns approval-state fixtures.
W09 owns boundary/redaction fixtures. W10 owns the pinned pilot fixture. W11
owns the mandatory full current-protocol gate.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | This goal exists to prove regression and release-gate behavior. |
