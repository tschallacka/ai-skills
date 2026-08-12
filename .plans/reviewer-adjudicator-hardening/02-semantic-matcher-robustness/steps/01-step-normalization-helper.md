# Step: 01-step-normalization-helper

## Ownership

- Goal: `02-semantic-matcher-robustness`
- Work unit: `W04`
- Type: `source`

## Change target

- File: `benchmark/planning/grade-blinded-run.sh`
- Primary symbol or file scope: `normalize tokens/ordinals/hyphens`
- Subscope: `N/A`

## Objective

§ 4.1
Add a shared normalizer that lowercases, strips hyphens and non-alphanumerics for word compare, and maps ordinal/digit forms (4/fourth, 3/third) so fourth generated button equals fourth-generated-button and generated button 4.

## Instructions

§ 5.1
Add a shared normalizer that lowercases, strips hyphens and non-alphanumerics for word comparison, and maps digit and ordinal forms (4/fourth, 3/third) to one canonical form; reuse it in signal and correction matching.

## Acceptance criteria

§ 6.1
fourth-generated-button, FOURTH generated button, and generated button 4 all reduce to the same word set.

## Handoff

§ 7.1
W05 and W06 use the same normalizer so signal and correction are symmetric.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
