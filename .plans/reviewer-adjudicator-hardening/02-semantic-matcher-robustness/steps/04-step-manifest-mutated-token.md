# Step: 04-step-manifest-mutated-token

## Ownership

- Goal: `02-semantic-matcher-robustness`
- Work unit: `W16`
- Type: `source`

## Change target

- File: `benchmark/planning/seed-blinded-defects.sh`
- Primary symbol or file scope: `defect-map manifest old/new fields`
- Subscope: `N/A`

## Objective

§ 4.1
Persist the mutated old and new tokens into each defect-map manifest entry so the grader can require the finding to reference the mutated conflict; keep expected_signal and required_correction unchanged. Frozen replay sources old/new from pilot-blinded-defects.json.

## Instructions

§ 5.1
In seed-blinded-defects.sh, extend the manifest.append block (lines 65-70) to also record the mutated old and new tokens for each defect; keep expected_signal and required_correction unchanged.

## Acceptance criteria

§ 6.1
Each manifest entry includes old and new alongside expected_signal and required_correction; the seed script still produces the encrypted defect map and target snapshot.

## Handoff

§ 7.1
W05 reads old/new from the manifest; W14 frozen replay sources old/new from pilot-blinded-defects.json for the archived runs.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
