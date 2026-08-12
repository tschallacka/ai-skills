# Step: 01-step-align-consumer-interface

## Ownership

- Goal: `16-manifest-consumer-interface`
- Work unit: `W78`
- Type: `source`

## Change target

- File: `planning/scripts/plan-env.sh`
- Primary symbol or file scope: `validated consumer CLI`
- Subscope: `N/A`

## Objective

Align the implemented CLI with the documented `check` plus `path` consumer
contract.

## Instructions

Use the actual `check` plus `path` flow. Ensure documentation says trusted
callers validate both manifests before sourcing and that all output is bounded.

## Acceptance criteria

The CLI, step documentation, and tests describe the same commands and safety
behavior.

## Handoff

W79 tests the final interface.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
