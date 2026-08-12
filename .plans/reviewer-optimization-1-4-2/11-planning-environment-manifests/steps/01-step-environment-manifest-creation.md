# Step: 01-step-environment-manifest-creation

## Ownership

- Goal: `11-planning-environment-manifests`
- Work unit: `W67`
- Type: `source`

## Change target

- File: `planning/scripts/create-plan.sh`
- Primary symbol or file scope: `environment manifest creation`
- Subscope: `N/A`

## Objective

Create deterministic global and plan-local environment manifests during plan creation.

## Instructions

1. Create or refresh `~/.plans/.env` with `PLANS_ROOT`, `PLANNING_SKILL_ROOT`, `PLANNING_SCRIPTS_ROOT`, and `PLANNING_TESTS_ROOT`.
2. Create or refresh `.plans/<plan-name>/.env` with `PLAN_ROOT`, `PLAN_NAME`, `PLAN_PROGRESS_FILE`, `PLAN_WORK_UNIT_INVENTORY`, `PLAN_VALIDATION_FILE`, `PLAN_CONTEXT_ROOT`, and `PLAN_STEPS_ROOT`.
3. Emit only shell-safe quoted assignments with canonical absolute paths. Do not include secrets, inherited arbitrary environment variables, command substitutions, or unquoted expansions.
4. Write atomically through a temporary file and rename, set mode `600`, and refresh only the owned manifest without deleting unrelated plan files.
5. Ensure plan creation works when the global `.plans` directory or plan directory does not yet exist.

## Acceptance criteria

- Both manifests are created by the normal plan-creation flow.
- Required variables are present, stable, correctly rooted, quoted, and non-secret.
- Refresh is deterministic and atomic; permissions are restrictive.
- No manifest is counted as a plan deliverable or copied into a published benchmark archive.

## Handoff

Hand off the exact variable contract and manifest paths to W68 and W69.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
