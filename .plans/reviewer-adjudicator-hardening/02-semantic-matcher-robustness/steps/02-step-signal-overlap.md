# Step: 02-step-signal-overlap

## Ownership

- Goal: `02-semantic-matcher-robustness`
- Work unit: `W05`
- Type: `source`

## Change target

- File: `benchmark/planning/grade-blinded-run.sh`
- Primary symbol or file scope: `signal_matches mutation-conflict + token-overlap`
- Subscope: `N/A`

## Objective

§ 4.1
Give signal matching a token-overlap fallback symmetric with correction, and require a true positive to reference the mutated conflict using the defect old/new tokens now carried by the manifest (W16), falling back to an explicit inconsistency indicator only when a mutation token is absent; minimum token-overlap floor for short signals.

## Instructions

§ 5.1
In grade-blinded-run.sh, change signal_matches to use the normalizer, add a token-overlap fallback with a minimum absolute overlap floor for short expected token sets, and add a mutated-conflict requirement: a true positive must reference at least one mutated token from the manifest old/new fields (present after W16); fall back to an explicit inconsistency indicator only when a mutation token is absent. Add old/new to the required validation list at line 53.

## Acceptance criteria

§ 6.1
Iterative AR-01 passes signal overlap for SD-01 and SD-03; a bare echo of one value without a mutated token does not pass SD-02; the required-field validation accepts old/new.

## Handoff

§ 7.1
W09 uses the classified result; W12 asserts positive and negative signal cases including the mutated-conflict rule; W14 frozen replay uses pilot-blinded-defects.json old/new.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
