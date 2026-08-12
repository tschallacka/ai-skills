# Step: 10-step-pilot-thresholds

## Ownership

- Goal: `07-pilot-release`
- Work unit: `W59`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `fixed pilot matrix and acceptance thresholds`
- Subscope: `N/A`

## Objective

§ 4.1
Run exactly one iterative and one fresh-review control for the current working-tree protocol using fixed task inputs, isolated roots, protocol metadata, and fail-closed token/latency/defect-detection thresholds.

## Instructions

§ 5.1
Apply thresholds only to compatible archived evidence and current-protocol runs. Mark missing or incompatible historical denominators unavailable; do not infer, backfill, or rewrite older reports. Reject adoption when required current evidence is absent.

## Acceptance criteria

§ 6.1
Pass only if all four runs have mandatory telemetry, no taint causes, complete archive/evidence sets, final fresh-review approval, iterative total tokens are lower than fresh control, iterative latency does not exceed control by more than 10%, and oracle true-positive and independent-catch rates are at least control; unavailable metrics fail adoption.

## Handoff

§ 7.1
Hand off the two normalized current-protocol run IDs and archive paths—iterative and fresh-review—plus task hashes, telemetry completeness, oracle.json paths, comparison.json, threshold results, and fail-closed reasons to W38. Historical archives remain contextual only.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
