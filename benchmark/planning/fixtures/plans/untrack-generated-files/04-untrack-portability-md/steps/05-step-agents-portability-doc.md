# Step: 05-step-agents-portability-doc

## Ownership

- Goal: `04-untrack-portability-md`
- Work unit: `W29`
- Type: `docs`

## Change target

- File: `AGENTS.md`
- Primary symbol or file scope: `PORTABILITY.md pointer`
- Subscope: `N/A`

## Objective

§ 4.1
Reword the loading-skills pointer from read-PORTABILITY.md to generate-then-read, and drop the stale hand-edit warning now that there is nothing committed to hand-edit.

## Instructions

§ 5.1
In AGENTS.md, reword the PORTABILITY.md pointer in the loading-skills prose: the catalogue is generated on demand (./generate-portability.sh) from portability-rules.json and is never committed; replace the stale hand-edit/regenerate-conflict warning with the generate-then-read instruction.

## Acceptance criteria

§ 6.1
AGENTS.md no longer instructs anyone to hand-resolve PORTABILITY.md conflicts or to treat it as a tracked file; the generate-then-read instruction is present.

## Handoff

§ 7.1
No downstream reliance: the wording is the user-facing record.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
