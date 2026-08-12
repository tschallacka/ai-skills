# Adversarial review: reviewer-adjudicator-hardening

## Review scope

§ 1.1
- Request: Harden the current 1.4.2 blinded adjudicator so its grader awards accurate, explainable detections of seeded defects without weakening fail-closed, blinding, or public redaction, and so thresholds/state are clean.
- Repository and context inspected: benchmark/planning/grade-blinded-run.sh (matching predicates candidate_path, location_matches, signal_matches, correction_matches, classification), setup-benchmark.sh (threshold exports ~179-180, reviewer-state synthesis ~1031-1136, reviewer prompt generation ~771-790, approval schema validator ~495-570), run-benchmark.sh (line 208), review-oracle.sh, seed-blinded-defects.sh, tests/test-review-oracle.sh, test-review-lifecycle.sh, tests/fixtures/, and the frozen archives under benchmark/results/. Background: .plans/reviewer-optimization-1-4-2/analysis-deep-dive.md.

## Findings

| ID | Missing or over-broad item | Required plan change | Status |
|---|---|---|---|
| AR-01 | W05 mutated-conflict reads old/new that the decrypted defect map does not carry | W16 added: seed-blinded-defects.sh manifest persists the mutated old/new tokens; W05 reads them; frozen replay sources old/new from pilot-blinded-defects.json | ✅ resolved |
| AR-02 | W09 per-defect report keyed by seed ID leaks or conflicts with redaction; partial classification unaccounted | W09 keeps per-defect detail in private rows and publishes a sanitized projection (ordinal, public finding ids, failed predicates, no SD ids); partial folds as not-true-positive with its own count key; redaction assertions updated | ✅ resolved |
| AR-03 | W02 location fallback underspecified; iterative AR-01 location lacks filename/section | W02 specifies the field set (location plus summary/observed_contradiction/evidence), filename-to-section resolution, and minimum acceptance so line/prose citations resolve | ✅ resolved |
| AR-04 | Signal leniency could admit unrelated findings; decision unspecified | W05 pins: require a mutated token; add a minimum token-overlap floor for short signals; per-rule positive and negative fixtures in W12 | ✅ resolved |
| AR-05 | W10 per-defect never reaches metadata; no state-builder target named | W10 targets the setup-benchmark.sh state builder (1031-1136) and threads grader to state to metadata after W08 | ✅ resolved |
| AR-06 | test-frozen-replay.sh is a phantom target with unknown discovery | W14 creates test-frozen-replay.sh (new file) with reviewer-B glob discovery, reads findings from approved_findings only, and pins iterative 3/3 / fresh 2/3 | ✅ resolved |
| AR-07 | W01 path gate can admit false positives | W01 is a candidate selector only; final classification is gated by W05/W06; a negative multi-path fixture stays false_positive in W12 | ✅ resolved |
| AR-08 | No per-goal regression gate; counts-dict integrity | W15 runs the existing consolidated fixture after W02 and the counts-dict assertion after W06; W09 preserves aggregate counts plus the partial key | ✅ resolved |
| AR-09 | W03 prompt wording could alter reviewer behavior | W03 touches only schema-validator comments plus a non-behavioral documentation note; no citation-style line is added to the reviewer prompt | ✅ resolved |
| AR-10 | W07 threshold source undefined; no rollback path | W07 exports SEMANTIC_THRESHOLD/INDEPENDENT_THRESHOLD defaults 1.0; W15 adds diagnosis/rollback via the W09 failed-predicate list | ✅ resolved |
| AR-11 | Inventory/goal prose and dependency hygiene | owned-work-units prose cleaned; W05 dependencies set to W04,W16; W09 testing companion scoped to fixtures; W14 transitive dependencies documented | ✅ resolved |

No additional substantive finding remains.

## Verdict

- Status: `✅ approved`
- Rationale: All eleven adversarial findings were incorporated into the plan (work units W01-W16, dependencies, steps, testing companions, and the definition-of-done coverage). No unresolved, open, or in-progress finding remains; the plan can proceed to validation and progress trackers.
