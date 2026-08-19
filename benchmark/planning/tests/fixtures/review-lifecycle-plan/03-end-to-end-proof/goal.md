# Goal: End-to-end seeded-oracle proof

## Current state and prior-goal handoffs

§ 2.1
The direct oracle test already covers semantic matching and redaction, but no test exercises the setup adapter that caused the production miss. The prior live run therefore passed local contracts while failing the integrated reviewer-to-oracle path.

## Outcome and definition of done

§ 3.1
Add integration coverage that exercises approval serialization, blinded semantic grading, lifecycle state, and fail-closed behavior so a real consolidated finding is counted and malformed evidence cannot appear as a false miss.

## Why this goal is needed

§ 4.1
Only an end-to-end fixture can prove that a reviewer finding remains gradeable after serialization, lifecycle selection, archive publication, and independent adjudication. It also prevents future direct-unit-test green results from masking adapter regressions.

## Scope

§ 5.1
In scope are direct consolidated semantic grading, setup adapter/lifecycle assertions, malformed finding cases, redaction, provenance, one iterative control, one fresh control, and complete validation. Out of scope are backward compatibility, legacy protocol changes, and application/UI testing.

## Affected files, systems, data, and interfaces

§ 6.1
W08 and W11 update test-review-oracle.sh fixtures; W09, W12, and W14 update test-review-lifecycle.sh fixtures; W16 updates the setup adapter’s test-only injection seam. W10 is the bounded command verification that consumes all preceding evidence.

## Dependencies and handoffs

§ 7.1
W08 depends on W02. W09 depends on W05, W06, and W15. W16 depends on W05 and W13; W14 depends on W01, W05, W06, W15, and W16 plus deterministic approval/seed inputs. W10 depends on W08, W09, W14, and W16 plus all required package validators and current-protocol archive checks.

## Implementation approach, risks, and edge cases

§ 8.1
First assert one complete AR-01 finding matches all three seeded defects and secrets remain redacted. Then exercise the actual setup adapter with deterministic capsules, approval artifacts, seed metadata, and archive publication while checking evidence/state/provenance outputs. Finally run the bounded matrix and accept only a complete attributable archive with 3/3 semantic and independent catches; otherwise preserve explicit fail-closed evidence.

## Owned work units

§ 9.1
`W08` — Test the complete consolidated semantic fixture and redaction.

§ 9.2
`W09` — Test lifecycle adapter assertions.

§ 9.3
`W10` — Run the bounded seeded benchmark and completion gate.

§ 9.4
`W14` — Test the actual setup adapter integration with deterministic capsule and seed inputs.

§ 9.5
`W16` — Add the test-only reviewer-command injection seam required by the adapter integration fixture.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The goal owns direct oracle, lifecycle integration, and bounded benchmark verification units. |

## Goal handoff

§ 10.1
W08 proves the complete consolidated finding is semantically gradeable and redacted; W09 proves lifecycle and malformed-result behavior; W14 proves the actual setup adapter preserves the evidence; W10 consumes those artifacts for the bounded fail-closed gate.

## Goal-size exception

§ 11.1
No exception: the goal owns five proof units.
