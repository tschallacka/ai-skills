# Step: 02-step-location-normalization

## Ownership

- Goal: `01-grading-contract-alignment`
- Work unit: `W02`
- Type: `source`

## Change target

- File: `benchmark/planning/grade-blinded-run.sh`
- Primary symbol or file scope: `location_matches`
- Subscope: `N/A`

## Objective

§ 4.1
Normalize section markers (section/sec/3.1/#/:) and accept line- or prose-location citations that reference the defect file and section; keep exact S-location equality passing.

## Instructions

§ 5.1
In grade-blinded-run.sh, change location_matches to consult finding location plus summary, observed_contradiction, and evidence. Normalize section markers (section/sec/S/3.1/#/:) and drop stray dots. Add filename-to-section resolution: when a finding names the defect file without a section token, require the section (or a clean line citation mapped to it) to appear in the same or referenced fields. Minimum acceptance: defect filename present in location OR any listed field, and the section resolvable.

## Acceptance criteria

§ 6.1
AR-01 iterative (location line 14... with filename/section only in evidence) resolves to plan-description.md 3.1; fresh AR-02/AR-03 prose locations resolve via plan-description.md; section 3.1, sec 3.1, 3.1, and plan-description.md 3.1 all pass.

## Handoff

§ 7.1
W03 documents the contract; goal 02 reuses the same normalization for signal comparisons.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
