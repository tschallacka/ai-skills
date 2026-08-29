# Step: 03-step-update-analysis-contract

## Ownership

- Goal: `02-authority-and-provenance`
- Work unit: `W07`
- Type: `docs`

## Change target

- File: `benchmark/planning/analyzer-prompt.md`
- Primary symbol or file scope: `reviewer lifecycle interpretation section`
- Subscope: `N/A`

## Objective

§ 4.1
Define A as handoff-only, B as final authority, and require analysis to report schema failures, provenance, and adapter transformations without inferring success.

## Instructions

§ 5.1
Update the analyzer contract to treat A as handoff-only, B as final authority, malformed evidence as a schema failure, and adapter loss as a publication failure. Forbid inferred adoption from prose or labels.

## Acceptance criteria

§ 6.1
A generated comparison reports authority, schema, provenance, and semantic rates separately and retains all fail-closed reasons.

## Handoff

§ 7.1
Goal 03 consumes stable report fields for its end-to-end assertions.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
