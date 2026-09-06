# Step: 06-step-release-build-libs

## Ownership

- Goal: `02-untrack-compiled-libs`
- Work unit: `W12`
- Type: `source`

## Change target

- File: `installer/build-release.sh`
- Primary symbol or file scope: `collect() precondition`
- Subscope: `missing-library build step`

## Objective

§ 4.1
Build-if-missing the five compiled libs before the listed-file hard error, so a release build from a clean tree self-heals instead of failing on a generated row.

## Instructions

§ 5.1
In installer/build-release.sh, before the listed-file hard error in collect(), build-if-missing the five compiled libs via planning/scripts/build-plan-libs.sh. The hard error stays for any listed file still missing after that build - non-generated rows have no self-heal.

## Acceptance criteria

§ 6.1
A release build from a tree without libs succeeds and includes them; a tree missing a non-generated listed file still hard-errors with the existing message; the build-if-missing runs at most once per invocation.

## Handoff

§ 7.1
W21's reviewer generation relies on this step having established build-if-missing in collect().

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
