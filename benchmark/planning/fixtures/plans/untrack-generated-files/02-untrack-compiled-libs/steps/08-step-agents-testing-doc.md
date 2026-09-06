# Step: 08-step-agents-testing-doc

## Ownership

- Goal: `02-untrack-compiled-libs`
- Work unit: `W14`
- Type: `docs`

## Change target

- File: `AGENTS.md`
- Primary symbol or file scope: `Running tests section`
- Subscope: `N/A`

## Objective

§ 4.1
Document the bootstrap: a clean checkout gets built artifacts via the run-tests bootstrap or prepack, and the suite is safe to run on a tree that has never been built.

## Instructions

§ 5.1
In AGENTS.md, extend the Running tests section with the bootstrap fact: a clean checkout gets built artifacts from the run-tests bootstrap (and npm prepack), the suite is safe on a never-built tree, and generated files are never committed. Leave the PLANNING_CONTEXT_CACHE note and everything else untouched.

## Acceptance criteria

§ 6.1
The section states the bootstrap and the never-committed rule; no sentence in AGENTS.md still claims the compiled libs are tracked files.

## Handoff

§ 7.1
No downstream reliance: the bootstrap documentation is the user-facing record of W08's behaviour.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
