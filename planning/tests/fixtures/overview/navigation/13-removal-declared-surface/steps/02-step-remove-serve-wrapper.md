# Step: 02-step-remove-serve-wrapper

## Ownership

- Goal: `13-removal-declared-surface`
- Work unit: `W79`
- Type: `config`

## Change target

- File: `planning/scripts/overview-serve.sh`
- Primary symbol or file scope: `file removal`
- Subscope: `N/A`

## Objective

§ 4.1
Delete the serve wrapper that chose a runtime rung and passed the plan directory to it. The binary serves the artifact itself, so the wrapper has nothing left to choose between.

## Instructions

§ 5.1
Delete planning/scripts/overview-serve.sh. The rung selection it performed is replaced by the binary's own serve mode, so nothing takes over its job and no shim is left in its place. Its manifest row, map entry, requirement rows and test coverage each belong to a separate named unit; do not edit them here.

## Acceptance criteria

§ 6.1
The wrapper is absent from the working tree, and serving the overview has exactly one implementation, the binary. No script, launcher or alias resolves the wrapper name any more.

## Handoff

§ 7.1
W80 can drop the runtime any-of group knowing the only consumer of those rungs is gone, and W86 can rewrite the serve assertions against the binary rather than against a file that still exists.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
