# Plan: Harden the current 1.4.2 blinded adjudicator

## Current state

§ 2.1
The current 1.4.2 blinded seeded cohort (iterative 20260811T203343Z and fresh 20260811T205844Z) scored 0/3 semantic and independent catches although both Reviewer B sessions found the seeded defects. The two hardening plans (.plans/reviewer-oracle-hardening, .plans/reviewer-oracle-evidence-hardening) fixed oracle design and evidence transport but not the grader matching predicates. .plans/reviewer-optimization-1-4-2/analysis-deep-dive.md (verified replay of grade-blinded-run.sh plus Codex-SQLite transcript confirmation) attributes the 0/3 to RC-01 the path/location exact-match pre-filter rejecting natural consolidated or prose findings, RC-02 schema-valid findings not being gradeable, RC-03 substring-only signal matching without token/ordinal/hyphen equivalence, RC-05 a mislabeled MISSING_DENOMINATOR that actually means missing thresholds, and RC-04 a genuine fresh-mode miss of SD-01 (reviewer-side, surfaced by diagnosis). Authoritative transcripts: iterative Reviewer B found all three defects in AR-01; fresh Reviewer B found two (AR-02, AR-03).

## Desired outcome

§ 3.1
The adjudicator becomes accurate and explainable without weakening fail-closed or blinding. Re-running the grader against the two frozen approval.json archives (no archive edits) must yield iterative 3/3 and fresh 2/3 semantic detection, with per-defect detail naming each failed predicate and the candidate findings considered. Threshold configuration is recorded as first-class state; MISSING_DENOMINATOR no longer fires when the oracle reports a valid denominator. Public aggregate fields (semantic_true_positive_rate, independent_catch_rate, seeded_denominator) and all fail-closed invariants remain intact.

## Approach

§ 4.1
Goal 01 aligns the grading contract: bounded path/location matching plus a documented reviewer-evidence contract so schema-valid equals gradeable. Goal 02 hardens the semantic matcher: token/ordinal/hyphen normalization, a signal token-overlap fallback symmetric with correction, and a mutated-conflict requirement. Goal 03 fixes threshold/denominator state. Goal 04 adds per-defect explainable classification with a deterministic post-run report. Goal 05 extends regression fixtures and pins frozen-archive regrade expectations, then runs the existing oracle/lifecycle test suites and the plan validator.

## Scope

§ 5.1
In scope: benchmark/planning/grade-blinded-run.sh, review-oracle.sh, setup-benchmark.sh threshold/state synthesis, run-benchmark.sh runner wiring, the reviewer prompt/schema contract note, benchmark/planning/tests oracle and lifecycle suites, and fixture additions. Out of scope: changing reviewer observation behavior (the RC-04 reviewer-side fix is a separate plan; this plan only surfaces it diagnostically), editing or retrofitting legacy 1.3.1/1.4.1 archives, weakening fail-closed or blinding, and leaking seed text or defect IDs into any public report.

## Affected areas

§ 6.1
benchmark/planning/grade-blinded-run.sh (path/location/signal/correction predicates, per-defect classification, report schema), benchmark/planning/setup-benchmark.sh (oracle evidence adapter, reviewer prompt/schema contract note around lines 495-570 and 771-790, reviewer-state synthesis 1031-1136, threshold export 179-180), benchmark/planning/run-benchmark.sh (threshold export before setup invocation, line 208), benchmark/planning/review-oracle.sh (blinded dispatch unchanged), benchmark/planning/worker-prompt.md and generated reviewer prompt note, benchmark/planning/tests/test-review-oracle.sh and test-review-lifecycle.sh, benchmark/planning/tests/fixtures/.

## Constraints and decisions

§ 7.1
Plans live under /home/mdibbets/git/ai-skills/.plans (PLANS_ROOT set) consistent with the two prior hardening plans. All grader changes must preserve terminal/evidence integrity, encrypted-map and defective-file hash verification, malformed-envelope fail-closed, Reviewer B-only authority, and public-report redaction of expected_signal/required_correction/seed IDs. The grader must not rely on hidden defect IDs matching finding IDs. Frozen-archive regrade uses the archived approval.json and pilot-blinded-defects.json only; no archived file is edited.

## Risks and open questions

§ 8.1
Risk: over-lenient matching could admit unrelated findings; mitigate with the bounded path gate and the mutated-conflict requirement, pinned by regression fixtures. Risk: regrading frozen archives changes displayed detection rates; this is intentional and documented, and legacy archives stay unedited. Open: whether a reviewer-prompt canonical-location note is applied (default yes, non-behavioral documentation) vs only grader-side robustness; whether the signal requires both mutated and expected tokens or accepts an explicit inconsistency indicator (default requires the mutated token).

## UI classification

- UI affected: no
- Rationale: <why>

## Adversarial review

- Artifact: `adversarial-review.md`
- Status: ✅ approved
