# Step: 04-step-context-listing-guard

## Ownership

- Goal: `03-untrack-reviewer-md`
- Work unit: `W20`
- Type: `source`

## Change target

- File: `planning/scripts/plan-context-lib.sh`
- Primary symbol or file scope: `context source listing`
- Subscope: `N/A`

## Objective

§ 4.1
Add a stated reason line when the context source listing omits an absent REVIEWER.md: the existing -f guard already omits it silently, so the defect is silence, not a dangling path; the present case is unchanged.

## Instructions

§ 5.1
In planning/scripts/plan-context-lib.sh's context source listing, when REVIEWER.md is absent from the source root, omit its entry and print a stated reason line (absent, generate with generate-reviewer.sh) instead of listing a path and hash that do not exist; the present case is unchanged.

## Acceptance criteria

§ 6.1
Listing without the file shows the omission and reason, never a dangling path or empty hash; with the file present the listing matches today's output; no regression in the plan-context paging tests.

## Handoff

§ 7.1
Plan-context users on an unbuilt tree see the stated reason; nothing downstream depends on the wording.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
