# Step: 02-step-run-tests-ensure-built

## Ownership

- Goal: `02-untrack-compiled-libs`
- Work unit: `W08`
- Type: `source`

## Change target

- File: `run-tests.sh`
- Primary symbol or file scope: `suite bootstrap`
- Subscope: `N/A`

## Objective

§ 4.1
Before suite discovery, build-if-missing: when any of the five compiled libs is absent run scripts/build-plan-libs.sh; when REVIEWER.md is absent run scripts/generate-reviewer.sh (no-op while it is still tracked; kicks in after goal 03); when rjq is neither on PATH nor built, run bootstrap.sh (W37). Staleness detection stays with the tests, so the bootstrap never masks drift.

## Instructions

§ 5.1
In run-tests.sh, before suite discovery: when any of the five compiled libs is missing, run planning/scripts/build-plan-libs.sh (prod target), suppress its stdout on success, and fail with its stderr on build failure; when REVIEWER.md is missing, run planning/scripts/generate-reviewer.sh (no-op while tracked, active after goal 03); when rjq is neither on PATH nor present at the gitignored planning/bin/<triple>/ path, run bootstrap.sh (W37). Never rebuild or regenerate when files exist - staleness detection stays with the tests. Keep bash 3.2 and BSD-userland portability.

## Acceptance criteria

§ 6.1
With the five libs removed, ./run-tests.sh rebuilds them and the suite proceeds; with all five present, build-plan-libs.sh is not invoked (their mtimes are unchanged); a deliberately broken function file makes run-tests fail with the compiler's message, not a suite failure.

## Handoff

§ 7.1
W16 and W31 rely on this bootstrap being the single entry that makes a clean tree suite-green, including the rjq arm via W37.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
