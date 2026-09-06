# Step: 03-step-libs-test-build-and-verify

## Ownership

- Goal: `02-untrack-compiled-libs`
- Work unit: `W09`
- Type: `test`

## Change target

- File: `planning/tests/test-plan-libs-build.sh`
- Primary symbol or file scope: `freshness assertions`
- Subscope: `N/A`

## Objective

§ 4.1
Rewrite from compare-against-committed to build-and-verify: byte-compare two fresh prod builds for determinism, keep the 500-line cap, symbol-set, function-file sourceability and dev/prod target assertions, and keep the mid-suite rewrite writing the now-gitignored paths; a source file that disagrees with a fresh build must still fail.

## Instructions

§ 5.1
Rewrite planning/tests/test-plan-libs-build.sh: replace the compare-against-committed assertion with a determinism check that builds prod twice to temp paths and byte-compares them; keep the --check teeth probe semantics against that fresh comparison, the 500-line cap, the function-file sourceability assertions, the facade symbol-count floor and the dev/prod target behaviour tests on a copy; the mid-suite rewrites keep writing the now-gitignored paths. Fault-inject: drift one function file from a fresh build and confirm the test fails.

## Acceptance criteria

§ 6.1
Every retained assertion passes on a tree with no tracked libs; the injected drift fails the test naming the offending group; no assertion in the file reads a tracked lib path; the test still refuses to run without the lib sources.

## Handoff

§ 7.1
Goal 06's W35 fresh-build comparisons rely on the determinism contract this test enforces.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
