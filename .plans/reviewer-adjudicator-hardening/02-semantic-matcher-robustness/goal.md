# Goal: Semantic matcher robustness (signal/correction)

## Current state and prior-goal handoffs

§ 2.1
signal_matches is substring-only while correction_matches has a 50 percent token-overlap fallback. This asymmetry filtered the iterative SD-02 signal (the initial button vs one initial button) and is latent for hyphenation (fourth-generated-button vs fourth generated button) and ordinal/digit forms (4 vs fourth).

## Outcome and definition of done

§ 3.1
Give the grader symmetric, normalized semantic matching so paraphrase, hyphenation, and ordinal/digit forms do not hide a correctly found defect, while a mutated-conflict requirement prevents false positives. DoD: signal matching gains token/ordinal/hyphen normalization and a token-overlap fallback symmetric with correction; a finding is a true positive only when it also references the mutated conflict; correction matching keeps its overlap fallback; exact-phrase matching still works.

## Why this goal is needed

§ 4.1
Adds symmetric normalization and a mutated-conflict requirement so paraphrase cannot hide a correctly found defect and cannot inflate a false positive.

## Scope

§ 5.1
In: shared normalizer, signal token-overlap fallback, mutated-conflict requirement, correction parity review. Out: no change to path/location gating semantics.

## Affected files, systems, data, and interfaces

§ 6.1
benchmark/planning/grade-blinded-run.sh normalizer, signal_matches, correction_matches and the classification block (153-192).

## Dependencies and handoffs

§ 7.1
W04 builds the normalizer; W05 changes signal matching; W06 reviews correction parity. Downstream: W09 per-defect report and goal 05 W12/W14 fixtures.

## Implementation approach, risks, and edge cases

§ 8.1
The mutated-conflict requirement stops a finding that merely echoes the wrong value from counting; add a minimum-overlap guard for short expected token sets. Keep the existing 50 percent correction fallback.

## Owned work units

§ 9.1
`W04` — Add a shared normalizer that lowercases, strips hyphens and non-alphanumerics for word compare, and maps ordinal/digit forms (4/fourth, 3/third) so fourth generated button equals fourth-generated-button and generated button 4.

## Testing requirement

| Test required | Rationale |
|---|---|
| no | Semantic matcher changes are verified by goal 05 fixtures (W12, W14) that depend on W04-W06; this goal owns no test unit. |

§ 9.2
 — Give signal matching a token-overlap fallback symmetric with correction, and require a true positive to reference the mutated conflict using the defect old/new tokens (W16) or an explicit inconsistency indicator; minimum token-overlap floor for short signals.

§ 9.3
`W06` — Review correction_matches against the new normalizer and mutated-conflict rule for parity and add regression notes for the 50 percent token-overlap fallback on short expected corrections.

§ 9.4
`W16` — Persist the mutated old and new tokens into each defect-map manifest entry so the grader can require the finding to reference the mutated conflict; keep expected_signal and required_correction unchanged. Frozen replay sources old/new from pilot-blinded-defects.json.

§ 9.5
`W05` — Give signal matching a token-overlap fallback symmetric with correction, and require a true positive to reference the mutated conflict using the defect old/new tokens now carried by the manifest (W16), falling back to an explicit inconsistency indicator only when a mutation token is absent; minimum token-overlap floor for short signals.

## Goal-size exception

§ 11.1
<required only when this goal has one permitted work unit>
