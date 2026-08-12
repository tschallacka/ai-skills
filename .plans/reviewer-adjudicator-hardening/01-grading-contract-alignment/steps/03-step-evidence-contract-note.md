# Step: 03-step-evidence-contract-note

## Ownership

- Goal: `01-grading-contract-alignment`
- Work unit: `W03`
- Type: `docs`

## Change target

- File: `benchmark/planning/setup-benchmark.sh`
- Primary symbol or file scope: `reviewer prompt + approval schema contract note`
- Subscope: `N/A`

## Objective

§ 4.1
Document the reviewer-evidence contract (single-file path recommended when a defect is in one file, file section location recommended, prose accepted, consolidation valid) in the generated reviewer prompt and schema validator comments so schema-valid equals gradeable.

## Instructions

§ 5.1
In setup-benchmark.sh, add the reviewer-evidence contract only to the approval schema validator comments (lines 495-570) and a non-behavioral documentation note. Do not add any citation-style instruction to the reviewer prompt (771-790): grader tolerance, not prompt luck, must make schema-valid equal gradeable; prompt text keeps its existing Consolidated-findings-may-cover-multiple-defects wording.

## Acceptance criteria

§ 6.1
Schema-validator comments document the accepted single-file path, file-and-section location, and prose/line location forms; the generated reviewer prompt is unchanged.

## Handoff

§ 7.1
Documentation only; no reviewer behavior change; goal-05 fixtures assume multi-file and prose findings remain valid.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
