# Step: 06-step-skill-instructions

## Ownership

- Goal: `13-removal-declared-surface`
- Work unit: `W83`
- Type: `docs`

## Change target

- File: `planning/SKILL.md`
- Primary symbol or file scope: `plan overview section`
- Subscope: `N/A`

## Objective

§ 4.1
Correct the instructions that tell an agent to run render-plan-overview.sh or overview-serve.sh, naming the binary and its subcommands instead. This is the contract agents act on, so a stale instruction here is followed rather than noticed.

## Instructions

§ 5.1
In the plan-overview section of planning/SKILL.md, replace every instruction to run the renderer or the serve wrapper with the binary and its subcommands, and state what an agent does on a platform with no prebuilt artifact. Change only that section; the reference documentation is W84.

## Acceptance criteria

§ 6.1
planning/SKILL.md names neither removed script anywhere, and an agent following its overview instructions verbatim on a supported platform renders and serves the page successfully. On an unsupported platform it follows the stated fallback rather than a command that does not exist.

## Handoff

§ 7.1
W84 documents the same commands the contract now names, so the two cannot drift; goal 09 inherits a contract whose every named command resolves.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
