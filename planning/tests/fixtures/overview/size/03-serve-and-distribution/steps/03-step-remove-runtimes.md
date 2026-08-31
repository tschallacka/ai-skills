# Step: 03-step-remove-runtimes

## Ownership

- Goal: `03-serve-and-distribution`
- Work unit: `W16`
- Type: `source`

## Change target

- File: `planning/scripts/runtime`
- Primary symbol or file scope: `directory removal`
- Subscope: `N/A`

## Objective

§ 4.1
Delete the four server rungs, whose duplicated logic is where the served-page defects lived.

## Instructions

§ 5.1
Remove the planning/scripts/runtime directory: the python, node and perl servers, the socat handler, and the sections endpoint that sliced HTML with a pattern against a hardcoded id list. Remove that directory only. An earlier version of this instruction also removed the serve wrapper and the requires.tsv rows declaring those runtimes; adversarial finding AR-04 recorded that neither had an inventory row while this step certified that no other file changes. The wrapper is now W79 and the requirement rows are W80.

## Acceptance criteria

§ 6.1
The runtime directory is gone and no rung remains for anything to select between. The requirement rows still declaring those runtimes are expected to be present at this point; W80 removes them, and the probe-versus-declaration gate is run there rather than here.

## Handoff

§ 7.1
W79 can delete the wrapper knowing it has nothing left to dispatch to, and W80 can drop the declarations knowing no shipped file consumes those runtimes.

## Atomicity check

- [ ] This step owns exactly one inventory work unit.
- [ ] No other file, symbol, test target, or verification flow changes here.
- [ ] Any follow-on target has a separately named work unit and step.
