# Step: 06-step-codestyle-portability-doc

## Ownership

- Goal: `04-untrack-portability-md`
- Work unit: `W30`
- Type: `docs`

## Change target

- File: `CODE-STYLE.md`
- Primary symbol or file scope: `PORTABILITY.md references`
- Subscope: `N/A`

## Objective

§ 4.1
Update references so the catalogue is described as generated on demand from portability-rules.json, keeping it the contract for the scripts.

## Instructions

§ 5.1
In CODE-STYLE.md, update the PORTABILITY.md references so the catalogue is described as generated on demand from portability-rules.json while remaining the portability contract the scripts are held to; adjust any sentence that implies reading a committed copy.

## Acceptance criteria

§ 6.1
No CODE-STYLE.md sentence implies PORTABILITY.md is a tracked, freshness-gated file; the contract status of the catalogue is restated in terms of the registry plus generator.

## Handoff

§ 7.1
No downstream reliance: the wording is the maintainer-facing record.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
