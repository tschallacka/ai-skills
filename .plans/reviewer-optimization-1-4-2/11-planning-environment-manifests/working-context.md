# Working context: 11-planning-environment-manifests

## Current state

- `planning/scripts/plan-env.sh` creates and validates `~/.plans/.env` plus a
  plan-local `.env`; `PLANS_ROOT` can point tests at an isolated root.
- `planning/scripts/create-plan.sh` writes both manifests when it creates a
  plan, and the utility exposes `check`, `path`, and `print` commands.
- Manifests are generated with mode `0600`, are rejected when symlinked or
  weakly permissioned, and are checked for allow-listed assignments before
  any sourcing occurs.

## Helper inventory and exceptions

- `planning/scripts/create-plan.sh`: migrated; creates the shared manifests
  and uses the manifest utility as the canonical path contract.
- `planning/tests/test-plan-env.sh`: migrated test helper; validates before
  sourcing and uses `path` to load the shared variables.
- `planning/scripts/plan-context*.sh`, `add-*.sh`, `update-*.sh`, and
  `validate-plan.sh`: intentionally standalone. They receive an explicit plan
  directory and must remain usable for legacy plans that predate `.env`; they
  do not repeat global helper paths or create monitor processes.
- `benchmark/planning/*.sh` and temporary monitor commands: intentionally
  standalone. They operate isolated benchmark/result roots and must not source
  a user's ambient plan manifest or copy `.env` into published artifacts.

## Handoff

- Outcome: manifest creation, validated consumption, helper inventory, and
  focused regression coverage are implemented.
- Files/data/routes/assets: `planning/scripts/plan-env.sh`, the installer
  package rows, `create-plan.sh`, and `planning/tests/test-plan-env.sh`.
- Verification: `planning/tests/test-plan-env.sh` passes; installer manifest
  validation is rerun after the package-count update.
- Caveats: the plan's unrelated release gate remains blocked by the missing
  genuine blinded seeded-defect oracle; this goal does not manufacture that
  evidence.
