# Step: 01-step-harden-manifest-checks

## Ownership

- Goal: `15-manifest-path-security`
- Work unit: `W76`
- Type: `source`

## Change target

- File: `planning/scripts/plan-env.sh`
- Primary symbol or file scope: `manifest_check`
- Subscope: `N/A`

## Objective

Make manifest validation complete before sourcing.

## Instructions

Reject duplicate keys, expansions, non-canonical or foreign-root paths, weak
ownership, and mismatched derived values. Keep the allowlist and schema checks
strict and preserve actionable rejection codes.

## Acceptance criteria

Every generated path is checked against its declared root and no untrusted
assignment can execute or alter the resolved helper paths.

## Handoff

W77 proves all rejection cases with isolated fixtures.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
