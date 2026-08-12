# Step: 01-step-dynamic-plan-mutation

## Ownership

- Goal: `10-plan-integrity-and-execution-persistence`
- Work unit: `W64`
- Type: `source`

## Change target

- File: `planning/SKILL.md`
- Primary symbol or file scope: `validated dynamic plan mutation contract`
- Subscope: `N/A`

## Objective

Prevent newly discovered scope from being captured as an informal note instead of a complete plan unit.

## Instructions

1. Define that any new implementation, verification, or risk scope discovered during execution must be represented by an owning goal or existing goal step, a work-unit inventory row, dependencies, and a testing companion when behavior is verifiable.
2. Require the worker to update the applicable goal/progress/working-context/handoff artifacts and record why the new unit is in scope.
3. Require a final plan validator run after the mutation and adversarial-review coverage for the new unit before it can be marked complete or used as a release dependency.
4. Distinguish durable plan state from scratch notes: notes may capture discovery, but they do not satisfy the plan until the linked artifact chain exists and validates.

## Acceptance criteria

- The skill explicitly rejects note-only scope additions as complete plan state.
- A newly added unit has an owning goal, inventory row, step, dependencies, progress row, and applicable testing companion.
- The worker records validator output and adversarial-review coverage for the mutation.
- The guidance preserves a resumable handoff with a clear next action.

## Handoff

Hand off the mutation checklist and validator/review evidence to the monitor and release workflow.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
