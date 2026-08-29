# Plan: Reviewer-to-oracle evidence and lifecycle hardening

## Current state

§ 2.1
The 2026-08-11 current-protocol iterative archive 20260811T152024Z-old-plan-iterative recorded a seeded denominator of 3 and semantic/independent catch rates of 0.0. Reviewer B actually detected a consolidated AR-01 contradiction, but its approval object omitted path, precise location, and required correction. The benchmark adapter then reduced the finding to only an ID/classification envelope before oracle grading, so the oracle correctly counted 0/3. Reviewer A and Reviewer B approval artifacts also produced an authority conflict. The direct oracle fixture already proves complete consolidated findings can catch all three defects; the missing coverage is the approval-to-oracle integration path.

## Desired outcome

§ 3.1
The hardened protocol must preserve a complete AR-NN finding object from reviewer approval to blinded grading, reject malformed evidence explicitly, count one valid consolidated finding against all matching seeded defects, use Reviewer B as the only final approval authority, and publish verifiable seed, capsule, plan, approval, transcript, and archive provenance. A complete seeded benchmark must either demonstrate 3/3 semantic and independent catches or record an explicit fail-closed unavailable/failure state without inference.

## Approach

§ 4.1
First define one canonical finding envelope and update the reviewer prompt and worker contract. Then make the setup adapter lossless and validate the envelope before grading. Next isolate Reviewer B authority and publish immutable provenance. Finally extend direct and harness-level tests, run one bounded seeded benchmark matrix, and require normal and complete plan validation plus fresh adversarial approval.

## Scope

§ 5.1
In scope: benchmark/planning/setup-benchmark.sh approval serialization and reviewer lifecycle state, benchmark/planning/grade-blinded-run.sh finding validation and semantic matching, benchmark/planning/worker-prompt.md and analyzer-prompt.md protocol wording, benchmark/planning/tests/test-review-oracle.sh and test-review-lifecycle.sh, archive/provenance fields, consolidated-finding fixtures, malformed evidence cases, and bounded current-protocol verification. Out of scope: changing seeded defect semantics, weakening thresholds, restoring legacy compatibility, changing application/UI behavior, or trusting reviewer classification labels as oracle results.

## Affected areas

§ 6.1
The primary runtime surfaces are the approval-to-oracle adapter and reviewer-state calculation in benchmark/planning/setup-benchmark.sh, semantic envelope validation in benchmark/planning/grade-blinded-run.sh, reviewer instructions in benchmark/planning/worker-prompt.md, and analysis interpretation in benchmark/planning/analyzer-prompt.md.

§ 6.2
The proof surfaces are benchmark/planning/tests/test-review-oracle.sh, benchmark/planning/tests/test-review-lifecycle.sh, their existing fixtures, the current 1.4.2 archive publication metadata, and the new plan evidence. No UI story applies because this initiative changes benchmark/reviewer infrastructure rather than a user-facing interface.

## Constraints and decisions

§ 7.1
Preserve the current 1.4.2 protocol and fail-closed adoption rule. Do not infer a true positive from classification prose, IDs, exit codes, or approval booleans. Reviewer A is handoff-only; Reviewer B is the final independent authority. A finding may consolidate multiple seeded defects, but every machine-readable field required for grading must be present.

## Risks and open questions

§ 8.1
The installed skill package is current at branch:nextupdate commit:803e75cb8cb0 but does not contain its referenced UI document; the repository-owned matching reference was read for completeness. The exact archive metadata fields may require coordinated producer/consumer updates. A fresh control may still time out at independent review; that is a bounded verification failure to record, never a reason to lower the gate.

## Environment facts

§ 9.1
Verify against the local benchmark fixture checkout in document order: adapter behavior in `benchmark/planning/setup-benchmark.sh`, then worker prompt and reviewer contracts under `benchmark/planning/`. No auth route; assertions run offline on fixture artifacts.

## Approach decisions

§ 10.1
Reviewer contracts live in `planning/` (shipped with the skill, byte-consistent with the generated `REVIEWER.md`); benchmark-specific expectations live under `benchmark/planning/`. The oracle's evidence stays on disk under `benchmark/results/` so the harness re-runs are reproducible without regenerating reviewer output.

## UI classification

- UI affected: no
- Rationale: This initiative changes benchmark/reviewer infrastructure and machine-readable evidence, not a user-facing interface or browser flow.

## Adversarial review

- Artifact: `adversarial-review.md`
- Status: ✅ approved
