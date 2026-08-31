# Step: 03-step-drop-runtime-requirements

## Ownership

- Goal: `13-removal-declared-surface`
- Work unit: `W80`
- Type: `config`

## Change target

- File: `planning/requires.tsv`
- Primary symbol or file scope: `overview-server-runtimes group`
- Subscope: `N/A`

## Objective

§ 4.1
Remove the four-row any-of group that declared python3, node, perl and socat for the overview server. A shipped binary asks nothing of the box, so the requirement is not merely satisfied differently, it is gone.

## Instructions

§ 5.1
Remove the four rows in planning/requires.tsv carrying the group id overview-server-runtimes, which declare python3, node, perl and socat as soft requirements. Their why column names the deleted wrapper, so the declaration is stale as well as unnecessary. Change nothing else in the file, and do not regenerate the installer here: install.sh is W113's change target, and an earlier version of this instruction told an executor to regenerate it inside a step whose atomicity check certifies that no other file changes. Adversarial finding AR-28 recorded that contradiction.

## Acceptance criteria

§ 6.1
planning/requires.tsv declares bash, jq and openssl and nothing else, and no row carries the overview-server-runtimes group id. The generated installer is expected to disagree with its source at this point; W113 regenerates it and W48 is downstream of both. Installing from a regenerated installer on a box with none of the four runtimes produces no warning at all.

## Handoff

§ 7.1
W81 and W82 record what ships knowing the runtime directory is no longer a declared dependency, and goal 09 inherits a skill whose declared requirements are three tools rather than seven.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
