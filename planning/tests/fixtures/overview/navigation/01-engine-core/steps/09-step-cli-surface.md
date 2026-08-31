# Step: 09-step-cli-surface

## Ownership

- Goal: `01-engine-core`
- Work unit: `W101`
- Type: `source`

## Change target

- File: `src/plan-overview/src/main.rs`
- Primary symbol or file scope: `parse_args()`
- Subscope: `N/A`

## Objective

§ 4.1
The command-line surface the binary exposes: --plan-dir, --out, --refresh, --watch, --serve and --port, with the same meanings the removed wrapper and its runtime servers gave them. This is the contract the skill contract and the documentation both name, so it is decided here rather than discovered at execution.

## Instructions

§ 5.1
Implement the argument parser in src/plan-overview/src/main.rs accepting --plan-dir, --out, --refresh, --watch, --serve and --port, each with the meaning the removed wrapper and its runtime servers gave it, and each rejecting a missing or malformed value with a named error rather than a default. Record the resulting flag contract in the step so the skill contract and the documentation quote it rather than paraphrase it. Hand-write the parsing: adding an argument-parsing crate would break the no-dependencies property this plan rests on.

## Acceptance criteria

§ 6.1
Each flag is accepted with its documented meaning, an unknown flag and a missing value are both refused with a message naming the flag, and the recorded contract matches the parser exactly. cargo still reports zero dependencies.

## Handoff

§ 7.1
W83 and W84 have a flag contract to quote, W14 has the serve and port flags it is invoked through, and W58 has the watch flag. None of them has to guess the surface.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
