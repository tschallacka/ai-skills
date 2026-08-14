# Step: 02-step-browser-story-check

## Ownership

- Goal: `03-verification-handoff`
- Work unit: `W05`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `US-01 browser flow`
- Subscope: `N/A`

## Objective

§ 4.1
Run the cached direct-click browser story against button-chain.html and record pass/fail evidence.

## Instructions

§ 5.1
Open the implemented `button-chain.html` in an approved browser test environment and execute `ui-story-runs/US-01.md` exactly: click the current last button through the initial, first generated, second generated, third generated, and fourth generated controls. Do not use console, DOM mutation, storage edits, injected events, or direct API shortcuts.

## Acceptance criteria

§ 6.1
The story passes only if each nonterminal click appends exactly one button below the previous last button and the terminal click leaves only exact lowercase `finished` text inside a visible white border.

## Handoff

§ 7.1
The final handoff can state whether the user-visible behavior met the benchmark contract and can link any failure to `bugs.md`. On failure, mark `US-01` as `🐛 bug found`, record reproduction steps, actual result, evidence, and severity in `bugs.md`, create new `NN-investigate-...` and `NN-fix-...` goals with atomic work units, update `work-unit-inventory.md`, `ui-user-stories.md`, and progress trackers, then retest `US-01` and any dependent story before completion.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
