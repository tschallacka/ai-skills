# Step: 03-step-generator-check-determinism

## Ownership

- Goal: `04-untrack-portability-md`
- Work unit: `W27`
- Type: `source`

## Change target

- File: `generate-portability.sh`
- Primary symbol or file scope: `--check mode`
- Subscope: `N/A`

## Objective

§ 4.1
--check compares two fresh temp regenerations for determinism instead of diffing against a committed file, so the mode keeps working with nothing tracked.

## Instructions

§ 5.1
In generate-portability.sh, change --check to build twice to temp paths and compare those for determinism (stamp excluded), exiting 1 on mismatch, instead of diffing against a committed file; the default write mode is unchanged. Requires rjq on PATH as today.

## Acceptance criteria

§ 6.1
--check exits 0 on the stable registry from a tree with no PORTABILITY.md; the contract test's injections make it exit 1 where they did before; the help text no longer references a committed file.

## Handoff

§ 7.1
W28's coupling row calls the contract test, whose freshness arm relies on this --check semantics only for maintainer use.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
