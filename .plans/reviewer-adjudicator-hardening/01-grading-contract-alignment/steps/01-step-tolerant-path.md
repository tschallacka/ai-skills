# Step: 01-step-tolerant-path

## Ownership

- Goal: `01-grading-contract-alignment`
- Work unit: `W01`
- Type: `source`

## Change target

- File: `benchmark/planning/grade-blinded-run.sh`
- Primary symbol or file scope: `candidate_path + path filter`
- Subscope: `N/A`

## Objective

§ 4.1
Accept a defect file if it appears as any ; separated path segment or is resolvable from location or evidence; preserve exact single-file equality.

## Instructions

§ 5.1
In grade-blinded-run.sh, replace the candidate_path equality gate with a selector that splits path on ; , and newlines and retains a finding if any segment equals the defect path, or if the defect path equals the normalized finding path; keep the location-derived fallback when path is empty. This is a candidate selector only: classification still requires signal and correction to pass.

## Acceptance criteria

§ 6.1
AR-02/AR-03 (multi-file path containing plan-description.md) become candidates for plan-description.md; a US-01-only finding stays a candidate only for its own path; a multi-path finding about an unrelated defect still fails signal/correction in W05/W06.

## Handoff

§ 7.1
W02 consumes the candidate set; W12 adds a negative multi-path fixture naming plan-description.md but about an unrelated defect that must stay false_positive.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
