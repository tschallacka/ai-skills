# Step: 06-step-package-manifest

## Ownership

- Goal: `05-protocol-archive`
- Work unit: `W26`
- Type: `config`

## Change target

- File: `planning/V27-PACKAGE-MANIFEST.txt`
- Primary symbol or file scope: `1.4.2 installable package file list`
- Subscope: `N/A`

## Objective

§ 4.1
Extend the finite manifest for the v27 replacement package to include the changed contract, helpers, benchmark/oracle records, fixtures, and runner evidence.

## Instructions

§ 5.1
Enumerate the exact installable set from planning/ and the exact source-only set for benchmark/planning/, benchmark/results/, telemetry databases, capsules, and pilot evidence. Update planning/V27-PACKAGE-MANIFEST.txt only for installable planning files and record generated/source-only status for every row.

## Acceptance criteria

§ 6.1
The expected installable set is exactly the tab-separated source column of planning/V27-PACKAGE-MANIFEST.txt: planning/SKILL.md, planning/REVIEWER.md, planning/context-v27/*, planning/tests/fixtures/planning-context-v27/*, planning/tests/test-planning-context-v27-contract.sh, planning/tests/test-installer-manifest.sh, planning/V27-PACKAGE-MANIFEST.txt, planning/scripts/{add-coverage,add-goal,add-ui-story,add-work-unit,configure-ui-story-cache,create-adversarial-review,create-plan-progress,create-plan,create-progress,create-ui-story-run-cache,create-ui-validation,create-work-unit-inventory,plan-content,plan-context-lib,plan-context,generate-reviewer,plan-document-lib,update-plan-content,update-plan-progress,update-progress,update-step,validate-plan}.sh. Benchmark inputs/results/telemetry/capsules are the explicit source-only set.

## Handoff

§ 7.1
Record the changed target, command/output evidence, and unavailable or environment-specific results for the next dependency.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
