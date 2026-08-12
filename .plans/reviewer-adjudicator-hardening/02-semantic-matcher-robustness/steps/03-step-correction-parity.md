# Step: 03-step-correction-parity

## Ownership

- Goal: `02-semantic-matcher-robustness`
- Work unit: `W06`
- Type: `source`

## Change target

- File: `benchmark/planning/grade-blinded-run.sh`
- Primary symbol or file scope: `correction_matches parity`
- Subscope: `N/A`

## Objective

§ 4.1
Review correction_matches against the new normalizer and mutated-conflict rule for parity and add regression notes for the 50 percent token-overlap fallback on short expected corrections.

## Instructions

§ 5.1
Review correction_matches against the new normalizer and the mutated-conflict rule; add a minimum absolute token-overlap guard for expected corrections with few tokens.

## Acceptance criteria

§ 6.1
Correction matching agrees with signal for the frozen findings and does not admit one-word echoes as corrections.

## Handoff

§ 7.1
Correction and signal agree; W09 and W14 depend on this parity.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
