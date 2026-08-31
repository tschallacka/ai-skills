# Step: 01-step-remove-template

## Ownership

- Goal: `13-removal-declared-surface`
- Work unit: `W78`
- Type: `config`

## Change target

- File: `planning/templates/plan-overview.html.tmpl`
- Primary symbol or file scope: `file removal`
- Subscope: `N/A`

## Objective

§ 4.1
Delete the at-underscore token template. It exists only for the substitution pass the binary replaces, and leaving it behind invites a future reader to wire it back up.

## Instructions

§ 5.1
Delete planning/templates/plan-overview.html.tmpl. Nothing else in this step: the manifest row and the map entry that name it belong to W81 and W82, and the skill and documentation references belong to W83 and W84. Confirm before deleting that no file outside those four owners still reads the template, and if one does, report it rather than editing it here.

## Acceptance criteria

§ 6.1
The template file is absent from the working tree. A repository-aware lookup for readers of it returns only the owners named above, each of which has its own unit. No substitution pass exists anywhere that would need a token template to succeed.

## Handoff

§ 7.1
W81 and W82 can remove the manifest row and the map entry knowing the file they describe is genuinely gone rather than pending, so their freshness checks are measuring the tree and not anticipating it.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
