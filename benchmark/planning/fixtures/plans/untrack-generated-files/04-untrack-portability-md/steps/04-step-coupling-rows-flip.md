# Step: 04-step-coupling-rows-flip

## Ownership

- Goal: `04-untrack-portability-md`
- Work unit: `W28`
- Type: `config`

## Change target

- File: `coupling.tsv`
- Primary symbol or file scope: `rows 6-7 portability check commands`
- Subscope: `N/A`

## Objective

§ 4.1
Flip the two PORTABILITY.md rows from running ./generate-portability.sh --check against the tree to running the portability contract test, whose temp regeneration carries the same drift detection.

## Instructions

§ 5.1
In coupling.tsv, flip rows 6-7 (the PORTABILITY.md rows) from running ./generate-portability.sh --check against the tree to running planning/tests/test-portability-contract.sh, whose temp regeneration carries the same drift detection; keep the glob triggers on portability-rules.json and the generator unchanged.

## Acceptance criteria

§ 6.1
blast-radius.sh on a changeset touching portability-rules.json executes the new check command and surfaces its result; the row format stays valid for test-blast-radius.sh's parser.

## Handoff

§ 7.1
W31's suite run exercises the flipped rows through blast-radius.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
