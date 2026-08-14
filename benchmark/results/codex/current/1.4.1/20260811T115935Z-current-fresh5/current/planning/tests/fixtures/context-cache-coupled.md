@@ path=plan-description.md
# Plan: coupled fixture

## Current state

§ 2.1
Coupled deterministic fixture.
@@ path=01-coupled/goal.md
# Goal: coupled

## Current state and prior-goal handoffs

§ 2.1
Fanout fixture.
@@ path=01-coupled/steps/01-step-a.md
# Step: a

## Objective

§ 4.1
Shared dependency A.
@@ path=01-coupled/steps/02-step-b.md
# Step: b

## Objective

§ 4.1
Shared dependency B.
@@ path=work-unit-inventory.md
# Work-unit inventory

| ID | Type | File | Primary scope | Subscope | Intended change | Dependencies | Goal | Step |
|---|---|---|---|---|---|---|---|---|
| W01 | source | `shared` | shared | N/A | A | — | 01-coupled | 01-step-a |
| W02 | source | `shared` | shared | N/A | B | W01 | 01-coupled | 02-step-b |
