# Step: 03-step-specify-reviewer-schema

## Ownership

- Goal: `01-lossless-finding-contract`
- Work unit: `W03`
- Type: `source`

## Change target

- File: `benchmark/planning/setup-benchmark.sh`
- Primary symbol or file scope: `Reviewer prompt construction`
- Subscope: `N/A`

## Objective

§ 4.1
Tell Reviewer B exactly which finding fields are mandatory and reject ID-only or narrative-only approval evidence before oracle grading.

## Instructions

§ 5.1
Expand the generated Reviewer B prompt with the exact JSON-like finding envelope, explain consolidated findings, require precise correction text, and state that incomplete evidence is terminally invalid.

## Acceptance criteria

§ 6.1
The prompt and harness reject an approval containing only AR-01 or summary/evidence prose without path, location, contradiction, impact, and correction.

## Handoff

§ 7.1
W04 carries the same contract into the worker-facing protocol without weakening independent review.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
