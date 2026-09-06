# Step: 01-step-release-package-fresh-builds

## Ownership

- Goal: `06-package-test-gates`
- Work unit: `W35`
- Type: `test`

## Change target

- File: `tests/test-release-package.sh`
- Primary symbol or file scope: `byte-identity assertions`
- Subscope: `N/A`

## Objective

§ 4.1
Compare the tarball's plan-core-lib.sh (and siblings) against fresh build-plan-libs.sh output instead of repo copies, keeping the exactly-once, zero-lib-sources, zero-compiler and installability assertions.

## Instructions

§ 5.1
In tests/test-release-package.sh, change the byte-identity assertions for the compiled libs to compare the tarball copies against fresh build-plan-libs.sh output (built to temp) instead of repo copies; keep the exactly-once presence, zero-lib-sources, zero-compiler and installability assertions as they are.

## Acceptance criteria

§ 6.1
The test passes on a tree with no tracked libs; a tarball carrying a stale lib (inject one) fails the byte-identity assertion; the untouched assertions still fire as before.

## Handoff

§ 7.1
W31 and W32 depend on this gate passing against build-time reality.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step. VIOLATION: also touched installer/src/00-header.sh
