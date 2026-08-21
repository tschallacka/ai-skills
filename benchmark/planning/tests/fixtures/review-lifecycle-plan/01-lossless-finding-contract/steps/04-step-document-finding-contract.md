# Step: 04-step-document-finding-contract

## Ownership

- Goal: `01-lossless-finding-contract`
- Work unit: `W04`
- Type: `docs`

## Change target

- File: `benchmark/planning/worker-prompt.md`
- Primary symbol or file scope: `final Reviewer B handoff contract`
- Subscope: `N/A`

## Objective

§ 4.1
Make the worker-facing protocol require complete machine-readable AR-NN finding objects with precise repository-relative path/location, observed contradiction, impact, evidence, required correction, and boolean independence.

## Instructions

§ 5.1
Update the worker-facing seeded-review instructions so every approved finding is a complete object, consolidated findings are allowed, and the oracle—not the reviewer—assigns semantic classification.

## Acceptance criteria

§ 6.1
The worker contract names every required field and states that ID-only or preclassified true-positive evidence is invalid.

## Handoff

§ 7.1
Goal 02 can treat all received finding objects as schema-governed evidence.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
