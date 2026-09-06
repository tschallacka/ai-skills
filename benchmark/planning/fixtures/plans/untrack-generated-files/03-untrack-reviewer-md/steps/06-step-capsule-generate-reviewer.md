# Step: 06-step-capsule-generate-reviewer

## Ownership

- Goal: `03-untrack-reviewer-md`
- Work unit: `W22`
- Type: `source`

## Change target

- File: `benchmark/planning/setup-benchmark.sh`
- Primary symbol or file scope: `capsule assembly`
- Subscope: `reviewer document`

## Objective

§ 4.1
Generate REVIEWER.md into the capsule when absent instead of copying only if present, so every capsule - git-archive tags included - carries it.

## Instructions

§ 5.1
In benchmark/planning/setup-benchmark.sh, replace the copy-REVIEWER.md-if-present behaviour with generate-if-missing into the capsule, on both the live-tree and git-archive paths, so every capsule carries the reviewer contract.

## Acceptance criteria

§ 6.1
A tag-archive capsule contains a REVIEWER.md whose pinned hash matches the capsule's SKILL.md; a live-tree capsule likewise; a capsule worker start (start-worker.sh path) finds the file where worker-prompt.md says it is.

## Handoff

§ 7.1
W33's capsule assertion relies on both paths generating the file.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
