# Step: 02-step-plan-mutation-dispatcher

## Ownership

- Goal: `21-plan-mutation-helper-governance`
- Work unit: `W91`
- Type: `source`

## Change target

- File: `planning/scripts/plan-mutate.sh`
- Primary symbol or file scope: `mutation dispatcher`
- Subscope: `N/A`

## Objective

§ 4.1
Provide one dispatcher for supported durable plan mutations and helpers for missing companion/progress operations.

## Instructions

§ 5.1
Implement plan-mutate.sh dispatch for goal, work-unit, testing, progress, status, review, decomposition, rebuild, and validation commands with atomic writes and safe arguments; support batched invocation through a strict temporary command script.

## Acceptance criteria

§ 6.1
The dispatcher invokes existing helpers consistently; testing companions and progress rows are created without corrupting tracker tables.

## Handoff

§ 7.1
Hand the executable dispatcher to W92 and future plan workers.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
