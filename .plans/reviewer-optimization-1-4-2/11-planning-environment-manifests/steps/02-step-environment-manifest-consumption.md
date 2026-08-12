# Step: 02-step-environment-manifest-consumption

## Ownership

- Goal: `11-planning-environment-manifests`
- Work unit: `W68`
- Type: `source`

## Change target

- File: `planning/scripts/plan-env.sh`
- Primary symbol or file scope: `safe environment manifest consumption`
- Subscope: `N/A`

## Objective

Give trusted temporary scripts a short, validated way to load planning paths.

## Instructions

1. Provide explicit commands for locating the global and plan-local manifests and for validating their root, ownership, mode, required keys, and assignment syntax.
2. Source manifests only after validation; reject command substitutions, backticks, semicolons, newlines in values, unexpected keys, symlink escapes, world/group-readable modes, and paths outside the declared roots.
3. Support the explicit interface `plan-env.sh check`, `plan-env.sh path`, and `plan-env.sh print` with bounded, concise output. Trusted callers must run `check` before sourcing the files returned by `path`; there is no automatic or implicit source operation.
4. Inventory all applicable planning helper scripts, benchmark helpers, monitor helpers, and temporary validation scripts. Migrate scripts that derive or repeat these paths to consume the validated manifest variables instead; document each script that intentionally remains standalone and why.
5. Never source manifests automatically from shell startup and never copy them into published benchmark artifacts.

## Acceptance criteria

- A trusted temporary helper can source the validated manifests and use short variables instead of repeated absolute paths.
- Applicable planning/helper scripts use the shared variables for plan and helper paths, or have a documented standalone exception with a safety rationale.
- Missing, malformed, unsafe, foreign-root, or weak-permission manifests fail closed with actionable errors.
- Validation does not execute manifest content before the syntax/root checks pass.

## Handoff

Hand off the consumer interface, rejection codes, and safety evidence to W69 and future monitor helpers.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
