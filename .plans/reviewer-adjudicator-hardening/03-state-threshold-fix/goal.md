# Goal: Threshold and denominator state synthesis fix

## Current state and prior-goal handoffs

§ 2.1
run-benchmark.sh invokes setup-benchmark.sh without SEMANTIC_THRESHOLD or INDEPENDENT_THRESHOLD; setup-benchmark.sh exports empty strings (179-180); float() on empty raises ValueError so thresholds become None and reviewer-state adds MISSING_DENOMINATOR even though oracle.json reports denominator 3. Confirmed in reviewer-state.json, protocol-metadata.json, and telemetry.json for both runs.

## Outcome and definition of done

§ 3.1
Record thresholds as first-class config and stop labeling missing thresholds as MISSING_DENOMINATOR. DoD: run-benchmark.sh exports SEMANTIC_THRESHOLD and INDEPENDENT_THRESHOLD before setup; reviewer-state/protocol-metadata/telemetry record real threshold values or a distinct MISSING_THRESHOLDS reason; MISSING_DENOMINATOR fires only when the oracle denominator is actually absent, invalid, or zero; the fresh/iterative archives re-synthesize without MISSING_DENOMINATOR on a valid denominator.

## Why this goal is needed

§ 4.1
Removes a mislabeled fail-closed reason and makes thresholds explicit, per the analysis recommendation to end-to-end check threshold source, export path, and archive-state synthesis.

## Scope

§ 5.1
In: runner threshold export and state reason split. Out: no change to grader scoring or semantics.

## Affected files, systems, data, and interfaces

§ 6.1
benchmark/planning/run-benchmark.sh (setup invocation) and setup-benchmark.sh threshold export (179-180) and reviewer-state synthesis (1031-1136).

## Dependencies and handoffs

§ 7.1
W07 exports thresholds before setup; W08 splits reasons in state synthesis. Goal 05 W13 lifecycle test verifies, W15 runs the suite.

## Implementation approach, risks, and edge cases

§ 8.1
MISSING_DENOMINATOR must only fire when the oracle denominator is truly absent, invalid, or zero; otherwise MISSING_THRESHOLDS. Do not normalize away a real failure; do not silence MISSING_THRESHOLDS either.

## Owned work units

§ 9.1
 — Export SEMANTIC_THRESHOLD and INDEPENDENT_THRESHOLD (defaults 1.0) before invoking setup-benchmark.sh so the state synthesizer receives real non-empty values.

## Testing requirement

| Test required | Rationale |
|---|---|
| no | Threshold/denominator reason behavior is verified by goal 05 lifecycle test W13 which depends on W07/W08; this goal owns no test unit. |

§ 9.2
 — Split MISSING_DENOMINATOR from MISSING_THRESHOLDS: MISSING_DENOMINATOR only on a truly absent/invalid/zero denominator; MISSING_THRESHOLDS only when threshold values are absent; record real thresholds in state and metadata.

§ 9.3
`W07` — Export SEMANTIC_THRESHOLD and INDEPENDENT_THRESHOLD (from config or passed values) before invoking setup-benchmark.sh so the state synthesizer receives real values instead of empty strings.

§ 9.4
`W08` — Split MISSING_DENOMINATOR from MISSING_THRESHOLDS: MISSING_DENOMINATOR fires only when the oracle denominator is absent/invalid/zero; MISSING_THRESHOLDS fires only when threshold values are absent; record real thresholds in state and metadata.

## Goal-size exception

§ 11.1
<required only when this goal has one permitted work unit>
