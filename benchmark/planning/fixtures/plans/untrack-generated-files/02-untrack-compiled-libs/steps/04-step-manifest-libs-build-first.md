# Step: 04-step-manifest-libs-build-first

## Ownership

- Goal: `02-untrack-compiled-libs`
- Work unit: `W10`
- Type: `test`

## Change target

- File: `tests/test-skill-files-manifest.sh`
- Primary symbol or file scope: `compiled-library presence rule`
- Subscope: `N/A`

## Objective

§ 4.1
The five lib rows keep requiring presence on disk, but the test builds them first when missing, so prepack on a clean checkout passes without a committed copy.

## Instructions

§ 5.1
In tests/test-skill-files-manifest.sh, before the presence assertions for the five compiled-lib rows, build-if-missing them via planning/scripts/build-plan-libs.sh, so prepack on a clean checkout passes; present files are checked exactly as before (existence, no content re-verify here - the lib test owns correctness).

## Acceptance criteria

§ 6.1
On a tree with the libs removed the test builds and passes; with a lib present (even stale), the build step does not run and the presence rule passes; a listed-but-still-missing non-generated file keeps failing as before.

## Handoff

§ 7.1
W11's prepack chain can assume the manifest test self-heals a clean tree.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
