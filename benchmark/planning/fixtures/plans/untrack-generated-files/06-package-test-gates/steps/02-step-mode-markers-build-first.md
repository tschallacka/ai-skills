# Step: 02-step-mode-markers-build-first

## Ownership

- Goal: `06-package-test-gates`
- Work unit: `W36`
- Type: `test`

## Change target

- File: `tests/test-mode-markers.sh`
- Primary symbol or file scope: `generated-file scan list`
- Subscope: `N/A`

## Objective

§ 4.1
Build-if-missing the compiled libs and generate-if-missing REVIEWER.md before the marker scans, so the marker contract holds on a tree that has never been built; scans keep failing on a present-but-wrong file.

## Instructions

§ 5.1
In tests/test-mode-markers.sh, build-if-missing the five compiled libs and generate-if-missing planning/REVIEWER.md before the marker scans that read them, so the marker contract holds on a never-built tree; a present-but-wrong file must still fail its scan.

## Acceptance criteria

§ 6.1
The test passes on a clean tree with the generated files absent beforehand; an injected wrong MODE marker in a compiled lib fails the scan; the build/generate steps run at most once per invocation.

## Handoff

§ 7.1
W31's full-suite proof includes this gate; nothing beyond it relies on the wording.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
