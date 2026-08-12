# Analysis: current 1.4.2 plan failure and hardening history

## Executive conclusion

The original `reviewer-optimization-1-4-2` plan is implementation-complete and structurally valid, but its intended outcome—adopting the current 1.4.2 reviewer protocol—remains unapproved. The latest clean, seeded current-protocol pair confirms that the failure is substantive rather than merely archival:

- Iterative run `20260811T203343Z-clean-current-seeded-iterative`: the worker exited 0, but final validation failed, Reviewer B rejected the plan, and the independent oracle scored 0/3 semantic catches and 0/3 independent catches.
- Fresh-review control `20260811T205844Z-clean-current-seeded-fresh`: validation, structural checks, process audit, Reviewer B binding, and oracle completion all passed, but Reviewer B still rejected the plan and the oracle scored 0/3 semantic catches and 0/3 independent catches.

The clean control is the most important result. Because fresh review also caught none of the seeded defects, the evidence does not support adopting iterative review, nor does it support treating fresh review as an effective current-protocol gate. The system is correctly fail-closed.

## What the current failure means

The seeded oracle used three private mutations in `plan-description.md`:

1. `one initial button` was changed to `two initial buttons`.
2. `fourth generated button` was changed to `third generated button`.
3. `visible white border` was changed to `visible black border`.

Both current runs reached terminal, schema-valid, provenance-bound Reviewer B evidence. The oracle then independently adjudicated the evidence and found:

| Mode | Worker/archive state | Reviewer B | Seeded defects | True positives | False negatives | Independent catches | Semantic rate | Independent rate |
|---|---|---|---:|---:|---:|---:|---:|---:|
| Iterative | tainted because validation failed | rejected | 3 | 0 | 3 | 0 | 0.0 | 0.0 |
| Fresh review | accepted process-wise | rejected | 3 | 0 | 3 | 0 | 0.0 | 0.0 |

The oracle also reported zero mechanical exact-ID matches. That is a diagnostic only: the protocol intentionally permits a reviewer finding such as `AR-01` to cover several hidden defects. The authoritative result is the semantic score, and that score is also zero.

The reviewer did identify real contradictions in the mutated plans, but those findings were not classified as true positives for the seeded defects. The fresh control's Reviewer B identified:

- an off-by-one ambiguity in the UI story;
- a contradiction between one and two initial buttons;
- a contradiction between white and black completion borders.

The iterative Reviewer B identified overlapping plan contradictions and a pending adversarial-review/report-integrity problem. In both cases, the final approval was false. This means the review produced useful human findings, but the current task/plan shape and grading contract did not convert them into successful seeded-defect catches.

## Why the original plan is still failed

The original plan has two separate gates:

1. Completeness: implementation files, tests, package records, plan artifacts, and protocol machinery exist and validate.
2. Adoption: a current iterative/fresh comparison must have complete evidence, valid independent review, and acceptable seeded-defect detection.

The completeness gate passes: the plan validator reports 100 work units across 21 goals, and the plan progress trackers report 100%. That status does not imply adoption. The adoption gate remains closed because:

- `plan_approved=false` in both latest Reviewer B artifacts;
- `oracle_completed=true`, but both rates are 0.0 with denominator 3;
- the iterative archive additionally has `VALIDATION_FAILED` and is tainted;
- the current metadata records missing threshold values, so threshold configuration itself needs inspection;
- historical 1.3.1 and 1.4.1 results are context only and cannot be retrofitted into current-protocol proof.

The fresh control is especially decisive: it passed validation and process checks but still failed approval and detection. Therefore the current issue cannot be explained solely by iterative correction behavior, lifecycle taint, or missing terminal handoff.

## Hardening round one: semantic oracle and approval-state hardening

Plan: `.plans/reviewer-oracle-hardening`.

### Problem addressed

The first failure was partly an oracle design failure. Reviewer B had consolidated the three seeded contradictions into a complete human finding, but the adapter/oracle path expected hidden seeded IDs (`SD-01`–`SD-03`) to equal reviewer finding IDs. The reviewer used a legitimate finding ID such as `AR-01`, so exact-ID grading incorrectly produced zero. The same round also found that review completion, plan approval, oracle completion, and adoption were too easy to conflate.

### Changes and guarantees

The first hardening round:

- introduced a private semantic defect manifest and independent adjudication;
- allowed one complete reviewer finding to match multiple seeded defects;
- made path, location, contradiction, evidence, and required correction part of the semantic decision;
- separated `review_completed`, `plan_approved`, `oracle_completed`, and `adoptable`;
- made false approval gradeable but non-adoptable;
- added explicit fail-closed reasons for missing denominators, incomplete oracle output, approval rejection/conflict, taint, and threshold failure;
- added regression coverage for consolidated findings, missing approval, false approval, and oracle-role enforcement.

That round proved the direct semantic fixture could score a complete consolidated finding as 3/3. It did not yet prove that the live approval-to-oracle adapter preserved the complete finding envelope under the real benchmark lifecycle.

## Hardening round two: lossless evidence, authority, provenance, and adapter integration

Plan: `.plans/reviewer-oracle-evidence-hardening`.

### Problem addressed

The second failure was an evidence-transport and attribution problem. The live setup adapter serialized the reviewer finding too narrowly, reducing a consolidated finding to an ID/classification-shaped record before grading. It also aggregated approval artifacts from multiple roles, allowing a false Reviewer A/Reviewer B conflict. Finally, the archive did not expose enough linked hashes to prove exactly which plan, defect snapshot, reviewer capsule, approval, transcript, and lifecycle handoff were graded.

### Changes and guarantees

The second hardening round:

- preserved the complete finding envelope through approval serialization and oracle grading;
- validated malformed findings explicitly instead of silently dropping fields;
- made Reviewer B the sole final approval authority while keeping Reviewer A handoff-only;
- added exact session, capsule, mode, freshness, and manifest binding;
- recorded source-plan, defective-plan, target-snapshot, approval, capsule, transcript, lifecycle, and selection hashes;
- added actual setup-adapter integration tests rather than testing only a direct oracle fixture;
- added deterministic reviewer identity seams for reproducible lifecycle tests;
- added archive and provenance assertions, malformed-envelope cases, and fail-closed interruption behavior;
- verified the package and plan contracts with a fresh adversarial review.

The latest seeded archives show that these transport and attribution controls now work: Reviewer B binding is passed, approval schema is valid, provenance hashes are retained, the oracle reaches a terminal report, and the public archive contains no private key or plaintext defect map. The failure has moved downstream: the current reviewer behavior does not produce findings that the semantic adjudicator accepts for the three seeded contradictions.

## What is fixed versus what is not

### Demonstrated as fixed

- The oracle no longer depends on hidden defect IDs matching reviewer IDs.
- Consolidated findings are supported by the semantic contract.
- Reviewer B is selected as final authority.
- Reviewer A does not create an approval conflict merely by producing handoff evidence.
- Approval schema and reviewer identity binding are validated.
- Seed, target snapshot, approval, capsule, transcript, and lifecycle provenance are cross-linked.
- The oracle can complete and report a denominator rather than silently returning unavailable data.
- The archive and process audits are reproducible and fail closed on invalid evidence.

### Still failing or unresolved

- Both current reviewer modes scored 0/3 semantic and independent catches.
- Reviewer B rejected the plan in both modes.
- The iterative run failed final plan validation and is tainted.
- The fresh run passed mechanical validation but still failed the substantive review/oracle gate.
- The archived state includes `MISSING_DENOMINATOR` even while the oracle report contains denominator 3. This indicates a producer/consumer or threshold-state inconsistency that must be traced; it must not be normalized away.
- The configured semantic and independent thresholds appear as `null` in current metadata. The intended threshold source, export path, and archive-state synthesis need an explicit end-to-end check.
- The current seeded task may be overloading one small planning proof with several contradictory requirements, while the reviewer is spending attention on broader plan-quality issues. That is a hypothesis, not a proven causal explanation.

## What can be used for deeper analysis

### Primary machine-readable evidence

- [Iterative comparison](/home/mdibbets/git/ai-skills/benchmark/results/20260811T203343Z-clean-current-seeded-iterative/comparison.md) — complete lifecycle, taint, findings, provenance, and oracle state.
- [Iterative oracle](/home/mdibbets/git/ai-skills/benchmark/results/20260811T203343Z-clean-current-seeded-iterative/current/oracle.json) — 0/3 semantic and independent catches, three false negatives.
- [Iterative approval](/home/mdibbets/git/ai-skills/benchmark/results/20260811T203343Z-clean-current-seeded-iterative/current/reviewers/20260811T203343Z-clean-current-seeded-iterative-current-B-1786481534995424311/plan/approval.json) — authoritative Reviewer B findings and rejection.
- [Fresh comparison](/home/mdibbets/git/ai-skills/benchmark/results/20260811T205844Z-clean-current-seeded-fresh/comparison.md) — clean control outcome.
- [Fresh oracle](/home/mdibbets/git/ai-skills/benchmark/results/20260811T205844Z-clean-current-seeded-fresh/current/oracle.json) — 0/3 semantic and independent catches, three false negatives.
- [Fresh approval](/home/mdibbets/git/ai-skills/benchmark/results/20260811T205844Z-clean-current-seeded-fresh/current/reviewers/20260811T205844Z-clean-current-seeded-fresh-current-B-1786482768027444165/plan/approval.json) — authoritative fresh Reviewer B findings and rejection.
- Each archive's `reviewer-lifecycle.jsonl` — exact launch, handoff, termination, authority, and independence events.
- Each archive's `reviewer-state.json`, `protocol-metadata.json`, and `telemetry.json` — state synthesis and provenance aliases that can be compared for drift.
- Each archive's `worker.jsonl` and Reviewer B transcript — observable actions, findings, corrections, validation commands, and token usage. Analyze event sequences, not private chain-of-thought.

### Source and contract evidence

- `benchmark/planning/pilot-blinded-defects.json` — the exact three mutations and expected correction signals.
- `benchmark/planning/seed-blinded-defects.sh` — how the defective plan, encrypted map, and target snapshot are created.
- `benchmark/planning/grade-blinded-run.sh` — the deterministic semantic matching rules and classification boundaries.
- `benchmark/planning/setup-benchmark.sh` — authority selection, approval serialization, state synthesis, threshold propagation, and publication.
- `benchmark/planning/tests/test-review-oracle.sh` — direct semantic, malformed-envelope, redaction, and oracle regression cases.
- `benchmark/planning/tests/test-review-lifecycle.sh` — actual adapter integration, Reviewer B authority, binding, provenance, and lifecycle cases.
- The two hardening plans and their adversarial reviews — rationale and intended guarantees for each control.

## Recommended next analysis questions

1. **Finding-to-defect mapping:** For each Reviewer B finding, replay the grader's matching predicates by hand and record which predicate fails: path, location, expected signal, required correction, completeness, or independence. This will identify whether the problem is reviewer observation, wording, location precision, or adjudicator strictness.
2. **Reviewer input visibility:** Compare the mutated plan snapshot, reviewer capsule manifest, and approved references to confirm the reviewer actually received the defective `plan-description.md` and did not receive the private defect map or seed IDs.
3. **Attention allocation:** Compare the findings and transcript event sequence between iterative and fresh modes. Determine whether both modes focused on pre-existing plan contradictions while overlooking the seeded mutations, and whether the seeded mutations were mentioned but not serialized into approved findings.
4. **State synthesis:** Trace the exact variables that produce `semantic_threshold`, `independent_threshold`, `seeded_denominator`, and `MISSING_DENOMINATOR`. The archive must explain why a terminal oracle denominator of 3 coexists with a state reason named `MISSING_DENOMINATOR`.
5. **Validation causality:** Isolate the iterative validation failures from semantic detection. A run can be invalid for plan completeness while still providing gradeable detection evidence; reports must preserve both dimensions.
6. **Task-design sensitivity:** Use several pre-approved seeded fixtures with the same semantic defect types but different wording and locations. This tests whether the reviewer is sensitive to the contradiction class or only to exact phrasing.
7. **Consolidation sensitivity:** Compare one finding per defect, one finding covering all defects, and mixed consolidation. The oracle supports consolidation, but live reviewer behavior may not produce sufficiently precise consolidated evidence.
8. **Repeatability:** Run multiple fresh controls with fixed task, seed, model, and reviewer protocol before drawing conclusions about detection rates. One iterative/fresh pair is sufficient to fail adoption, but not sufficient to estimate a stable reviewer capability.

## Recommended next experiments

The next diagnostic run should not weaken the release threshold or edit the prior archives. Use a new isolated current-protocol cohort with:

- the same three-defect fixture;
- explicit non-null semantic and independent thresholds recorded in generated environment and final metadata;
- a reviewer-facing checklist that requires reconciling every top-level desired-outcome statement against goal, inventory, and UI-story evidence;
- a deterministic post-run report showing each defect, the candidate finding(s) considered, and the exact failed matching predicate, while keeping private seed IDs out of the reviewer capsule;
- at least one fresh-review repetition before comparing models, prompt variants, or task wording.

Any change to reviewer instructions or task design must be tested against the direct oracle fixtures and the actual adapter integration tests. A change is not a success merely because the worker receives `overall_plan_approval=true`; it must produce complete independent evidence and meet both configured semantic thresholds.

## Final disposition

The original plan should remain marked complete for implementation and verification work-unit bookkeeping, but adoption should remain explicitly **not approved**. The two hardening rounds successfully converted an opaque false zero into an attributable, reproducible zero: the current protocol now proves that the reviewer/oracle pipeline completed, preserved evidence, and still failed to detect the seeded contradictions. The next work is reviewer/task sensitivity analysis and threshold/state cleanup, followed by another current seeded pair only after those diagnostics are understood.
