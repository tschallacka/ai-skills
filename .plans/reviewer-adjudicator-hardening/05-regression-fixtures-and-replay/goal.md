# Goal: Regression fixtures and frozen-archive replay

## Current state and prior-goal handoffs

§ 2.1
tests/test-review-oracle.sh covers direct fixtures but only single-file and S-location shaped findings; no fixture models multi-file path, prose location, section variants, hyphen, or paraphrase, and no frozen-archive replay test exists.

## Outcome and definition of done

§ 3.1
Lock the new grader behavior with fixtures and prove it against the frozen archives. DoD: fixture set covers multi-file path, prose location, section/S variants, hyphen, paraphrase, and threshold reason split; re-grading the frozen approval.json archives yields iterative 3/3 and fresh 2/3 pinned as expectations; existing test-review-oracle.sh, test-review-lifecycle.sh, and the plan validator pass.

## Why this goal is needed

§ 4.1
Proves the hardened grader is accurate and non-regressive and pins the frozen-archive expectations (iterative 3/3, fresh 2/3) as regression locks without editing archives.

## Scope

§ 5.1
In: fixture expansion, threshold reason lifecycle case, frozen-replay test, full suite run plus plan validator. Out: no edits to frozen archives.

## Affected files, systems, data, and interfaces

§ 6.1
benchmark/planning/tests/test-review-oracle.sh, test-review-lifecycle.sh, new test-frozen-replay.sh, and tests/fixtures/.

## Dependencies and handoffs

§ 7.1
W12 fixtures consume W09/W05 behavior; W13 threshold case consumes W08; W14 frozen replay consumes W12; W15 runs all suites plus the plan validator and is the final gate.

## Implementation approach, risks, and edge cases

§ 8.1
The frozen-replay expectation must be deterministic; do not normalize away a failed predicate. Replay uses archived approval.json and pilot-blinded-defects.json only; archives stay unedited.

## Owned work units

§ 9.1
`W12` — Extend oracle fixtures to cover multi-file path, prose location, section/sec/S variants, hyphenated signal, paraphrase signal, and mutated-conflict positive and negative cases.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | This goal owns W12, W13, W14 (test) and W15 (verification). |

§ 9.2
`W13` — Add lifecycle cases asserting MISSING_THRESHOLDS when thresholds are absent and that MISSING_DENOMINATOR does not fire when the oracle reports a valid denominator.

§ 9.3
`W14` — Add a deterministic test that regrades the frozen approval.json archives against pilot-blinded-defects.json and pins iterative 3/3 and fresh 2/3 as expectations without editing archives.

§ 9.4
`W15` — Run test-review-oracle.sh, test-review-lifecycle.sh, test-frozen-replay.sh, the plan validator, and the oracle/lifecycle suites; all must pass.

## Goal-size exception

§ 11.1
<required only when this goal has one permitted work unit>
