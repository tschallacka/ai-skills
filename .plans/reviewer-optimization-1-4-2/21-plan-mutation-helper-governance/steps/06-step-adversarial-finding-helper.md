# Step: 06-step-adversarial-finding-helper

## Ownership

- Goal: `21-plan-mutation-helper-governance`
- Work unit: `W95`
- Type: `source`

## Change target

- File: `planning/scripts/add-adversarial-finding.sh`
- Primary symbol or file scope: `Add adversarial findings through a validated helper`
- Subscope: `N/A`

## Objective

§ 4.1
Provide atomic review-finding insertion with explicit status handling

## Instructions

§ 5.1
Validate the plan, finding identifier, safe one-line fields, and explicit status; insert the finding atomically before the review boundary and reject duplicates.

## Acceptance criteria

§ 6.1
A finding can be added as open, in-progress, or resolved; malformed, duplicate, or boundary-less reviews fail closed without partial writes.

## Handoff

§ 7.1
The review lifecycle can add and resolve AR findings through plan-mutate without direct review-file edits.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
