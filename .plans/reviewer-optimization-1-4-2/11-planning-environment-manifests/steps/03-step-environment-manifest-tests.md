# Step: 03-step-environment-manifest-tests

## Ownership

- Goal: `11-planning-environment-manifests`
- Work unit: `W69`
- Type: `test`

## Change target

- File: `planning/tests/test-plan-env.sh`
- Primary symbol or file scope: `environment manifest fixture tests`
- Subscope: `N/A`

## Objective

Prove the complete environment-manifest lifecycle and safety boundary.

## Instructions

Add isolated shell fixtures covering creation, refresh, required variables, quoting, mode `600`, plan isolation, safe sourcing, malformed input rejection, path-root rejection, temporary-helper use, and helper-script migration/exception inventory. Assert that manifests remain outside published plan/result artifact counts.

## Acceptance criteria

- All valid lifecycle fixtures pass.
- Every unsafe manifest fixture fails closed without executing injected content.
- Refresh is byte-stable and unrelated files survive.
- The test leaves no manifest or fixture artifacts in the repository or published benchmark archive.

## Handoff

Hand off the test command and full fixture output to the environment-manifest goal and final release gate.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
