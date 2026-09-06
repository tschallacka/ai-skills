# Step: 07-step-capsule-build-libs

## Ownership

- Goal: `02-untrack-compiled-libs`
- Work unit: `W13`
- Type: `source`

## Change target

- File: `benchmark/planning/setup-benchmark.sh`
- Primary symbol or file scope: `capsule assembly`
- Subscope: `compiled libraries`

## Objective

§ 4.1
Build the five libs into the capsule after copying planning/scripts/, on both the live-tree and git-archive paths, so tag-based runs are not silently missing generated files.

## Instructions

§ 5.1
In benchmark/planning/setup-benchmark.sh, after the capsule copy of planning/scripts/, build the five compiled libs into the capsule (run build-plan-libs.sh with the capsule as skill root or copy freshly built files), on both the live-tree path and the git-archive path, so tag-based capsules are never silently missing generated files.

## Acceptance criteria

§ 6.1
A capsule assembled from a tag archive contains the five built libs; a live-tree capsule contains them too; sourcing the capsule's plan-document-lib.sh exposes the facade symbol set (the same assertion test-plan-libs-build makes).

## Handoff

§ 7.1
W33's capsule assertions rely on both assembly paths carrying built libs.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
