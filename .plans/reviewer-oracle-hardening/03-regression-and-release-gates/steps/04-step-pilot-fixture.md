# Goal 03 / Step 04: Pin the pilot failure fixture

## Ownership

- Goal: `03-regression-and-release-gates`
- Work unit: `W10`
- Type: `data`

## Change target

- File: `benchmark/planning/tests/fixtures/pilot-consolidated-finding.json`
- Primary symbol or file scope: `pilot failure fixture`
- Subscope: `AR-01 semantic coverage`

## Objective

Preserve the known failure as deterministic source data independent of mutable
historical result archives.

## Instructions

Record three semantic defects, one redacted AR-01 finding covering all three,
the expected adjudication rows, and expected approval/adoption states. Do not
copy secrets, private paths, or mutable archive references.

## Acceptance criteria

The fixture alone reproduces the old 0/3 mechanical-versus-semantic mismatch
and the corrected expected 3/3 semantic coverage.

## Handoff

W07, W08, and W11 consume this immutable test fixture.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
