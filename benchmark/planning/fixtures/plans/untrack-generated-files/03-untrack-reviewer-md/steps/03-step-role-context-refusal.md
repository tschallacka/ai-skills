# Step: 03-step-role-context-refusal

## Ownership

- Goal: `03-untrack-reviewer-md`
- Work unit: `W19`
- Type: `source`

## Change target

- File: `planning/scripts/role-context.sh`
- Primary symbol or file scope: `REVIEWER.md read path`
- Subscope: `N/A`

## Objective

§ 4.1
When the skill tree has no REVIEWER.md, refuse with an actionable message naming generate-reviewer.sh and the skill root, instead of reading a missing file.

## Instructions

§ 5.1
In planning/scripts/role-context.sh, when the skill tree has no REVIEWER.md where its reads expect it, exit non-zero with an actionable message naming planning/scripts/generate-reviewer.sh and the skill root it looked under. Reads with the file present stay byte-identical. Passes shellcheck and bash 3.2.

## Acceptance criteria

§ 6.1
With REVIEWER.md absent, role-context.sh fails with the generator named in the message; with it present, output is unchanged; the refusal fires before any partial output.

## Handoff

§ 7.1
Review sessions started before the bootstrap runs get the named fix instead of a missing-file error.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
