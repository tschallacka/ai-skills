# Step: 02-step-run-final-artifact-audit

## Ownership

- Goal: `03-record-benchmark-evidence`
- Work unit: `W08`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `Isolated workspace final audit`
- Subscope: `N/A`

## Objective

§ 4.1
Audit only the benchmark workspace for expected plan output, mandatory non-empty artifacts, and absence of forbidden HTML/HTM files before completion.

## Instructions

§ 5.1
Inspect only the isolated benchmark workspace and selected plan directory. Verify mandatory plan artifacts are non-empty, ui-story-runs/US-01.md exists, at least one goal and testing companion exist, and no .html or .htm files were generated.

§ 5.2
Record the audit result in analysis-report.md and keep final tagged validator output in validation.md.

## Acceptance criteria

§ 6.1
The audit confirms all mandatory artifacts are present and non-empty, no forbidden HTML/HTM artifacts exist, and no unauthorized filesystem escape is recorded.

## Handoff

§ 7.1
The final response can cite analysis-report.md as the benchmark evidence artifact.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
