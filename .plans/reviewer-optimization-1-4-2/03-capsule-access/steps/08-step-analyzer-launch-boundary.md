# Step: 08-step-analyzer-launch-boundary

## Ownership

- Goal: `03-capsule-access`
- Work unit: `W45`
- Type: `source`

## Change target

- File: `benchmark/planning/run-benchmark.sh`
- Primary symbol or file scope: `analyzer launch block`
- Subscope: `N/A`

## Objective

§ 4.1
Create a fresh analyzer capsule containing only benchmark instructions, summary, and current run results, with no source checkout or previous result roots.

## Instructions

§ 5.1
Generate analyzer-manifest.json at `/tmp/ai-skills-capsules/<run-id>/analysis/analyzer-manifest.json`; launch `codex -a never exec --json -C "$ANALYZER_WORKSPACE" --sandbox workspace-write --add-dir "$ANALYZER_CAPSULE" --add-dir "$ANALYZER_WORKSPACE"` with current-run.tsv and the current result archives copied into the analyzer capsule. Do not add RUN_RESULTS_ROOT directly.

## Acceptance criteria

§ 6.1
The exact analyzer command binds ANALYZER_CAPSULE and ANALYZER_WORKSPACE only, reads CURRENT_RUN_INPUT, writes analyzer-manifest.json/audit.jsonl, and never receives RUN_RESULTS_ROOT or tagged source as an add-dir.

## Handoff

§ 7.1
Record the changed target, command/output evidence, and unavailable or environment-specific results for the next dependency.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
