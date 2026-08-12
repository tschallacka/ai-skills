# Step: 06-step-capsule-discovery

## Ownership

- Goal: `03-capsule-access`
- Work unit: `W43`
- Type: `discovery`

## Change target

- File: `benchmark/planning/setup-benchmark.sh`
- Primary symbol or file scope: `approved relative-reference resolver`
- Subscope: `N/A`

## Objective

§ 4.1
Enumerate every relative reference required by SKILL.md/REVIEWER.md and determine the minimal capsule file set before implementing the capsule copy boundary.

## Instructions

§ 5.1
Resolve every relative reference named by the tagged planning/SKILL.md and REVIEWER.md into a manifest with source path, destination path, reason, hash, and whether it is worker-only, reviewer-only, or analyzer-only. Fail if a reference cannot be resolved without exposing an unallowlisted parent.

## Acceptance criteria

§ 6.1
The manifest resolves to the canonical worker/reviewer/analyzer roots, lists every copied file and hash, and fails closed when a relative reference points outside the tagged planning directory.

## Handoff

§ 7.1
Record the changed target, command/output evidence, and unavailable or environment-specific results for the next dependency.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
