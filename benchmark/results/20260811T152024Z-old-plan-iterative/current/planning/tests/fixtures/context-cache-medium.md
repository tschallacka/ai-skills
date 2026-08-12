@@ path=plan-description.md
# Plan: medium fixture

## Current state

§ 2.1
Medium deterministic fixture.
@@ path=01-first/goal.md
# Goal: first

## Current state and prior-goal handoffs

§ 2.1
Ready.
@@ path=01-first/steps/01-step-one.md
# Step: one

## Objective

§ 4.1
First step.
@@ path=02-second/goal.md
# Goal: second

## Current state and prior-goal handoffs

§ 2.1
Depends on first.
@@ path=02-second/steps/01-step-two.md
# Step: two

## Objective

§ 4.1
Second step.
@@ path=work-unit-inventory.md
# Work-unit inventory

| ID | Type | File | Primary scope | Subscope | Intended change | Dependencies | Goal | Step |
|---|---|---|---|---|---|---|---|---|
| W01 | source | `one` | one | N/A | First | — | 01-first | 01-step-one |
| W02 | source | `two` | two | N/A | Second | W01 | 02-second | 01-step-two |
