# Step: 07-step-worker-launch-boundary

## Ownership

- Goal: `03-capsule-access`
- Work unit: `W44`
- Type: `source`

## Change target

- File: `benchmark/planning/setup-benchmark.sh`
- Primary symbol or file scope: `generated start-worker.sh codex launch`
- Subscope: `N/A`

## Objective

§ 4.1
Replace full tagged-source --add-dir access with the worker capsule and explicitly expose only the workspace plus approved capsule paths; record command/path audits.

## Instructions

§ 5.1
Generate worker-manifest.json at `/tmp/ai-skills-capsules/<run-id>/<revision>/worker/worker-manifest.json`; launch `codex -a never exec --json -C "$WORKER_WORKSPACE" --sandbox workspace-write --add-dir "$WORKER_CAPSULE" --add-dir "$WORKER_WORKSPACE"` with no SRC_ROOT path. Capsule entries are task-spec.md, tagged planning/SKILL.md, REVIEWER.md, and each approved relative reference, each hashed in the manifest.

## Acceptance criteria

§ 6.1
The exact worker command uses WORKER_CAPSULE and WORKER_WORKSPACE only, writes worker-manifest.json and audit.jsonl at the canonical roots, and maps every denied event to BB-ACCESS-DENIED before publication.

## Handoff

§ 7.1
Record the changed target, command/output evidence, and unavailable or environment-specific results for the next dependency.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
