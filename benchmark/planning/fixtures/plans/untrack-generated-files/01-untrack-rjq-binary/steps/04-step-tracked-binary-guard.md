# Step: 04-step-tracked-binary-guard

## Ownership

- Goal: `01-untrack-rjq-binary`
- Work unit: `W04`
- Type: `test`

## Change target

- File: `tests/test-shipped-binaries.sh`
- Primary symbol or file scope: `tracked-binary regression guard`
- Subscope: `N/A`

## Objective

§ 4.1
Add the guard that fails when git ls-files names any file under planning/bin or chat/bin, so a re-committed binary blob fails the suite rather than passing quietly.

## Instructions

§ 5.1
In tests/test-shipped-binaries.sh add one guard asserting git ls-files planning/bin and git ls-files chat/bin are both empty, failing with a message that names planning/MAINTAINER.md section 2.15 and lists the offending paths. Fault-inject it: stage a dummy file under planning/bin, confirm the guard fails, remove it. The guard requires git the same way the register tests do.

## Acceptance criteria

§ 6.1
The guard fails on the staged dummy and passes once it is removed; the failure message names section 2.15; the rest of the test's registry validation is untouched.

## Handoff

§ 7.1
W31's whole-plan ls-files assertion relies on this guard having pinned the never-tracked contract for planning/bin and chat/bin.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
