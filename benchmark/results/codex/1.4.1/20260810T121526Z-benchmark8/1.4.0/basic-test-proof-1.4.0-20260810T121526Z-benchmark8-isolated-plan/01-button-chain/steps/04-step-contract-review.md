# Step: 04-step-contract-review

## Ownership

- Goal: `01-button-chain`
- Work unit: `W05`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `future button-chain implementation contract review`
- Subscope: `N/A`

## Objective

§ 4.1
Check that the three implementation units specify one concrete HTML file, one initial button, current-last traversal, exact append placement/counts, terminal DOM invariant, and explicit white-border declaration before handing off to browser validation.

## Instructions

§ 5.1
Read the W01, W02, W03, W06, and W05 plan targets and verify their single-file ownership, dependency order, current-last DOM rule, exact append counts, terminal DOM invariant, and explicit non-zero solid white-border declaration. Record any inconsistency as a plan finding; do not create or inspect button-chain.html.

## Acceptance criteria

§ 6.1
The future implementation contract is internally consistent: W06 owns initialization, W03 owns only the click handler and consumes W01/W02/W06, W05 gates W04, and W04 can execute the named flow without inferring a selector or terminal invariant. No HTML artifact is created during this proof.

## Handoff

§ 7.1
W04 receives a reviewed contract and the four-order US-01 cache.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
