# Step: 03-step-contract-proof

## Ownership

- Goal: `01-button-chain-contract`
- Work unit: `W07`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `future button-chain contract review`
- Subscope: `N/A`

## Objective

§ 4.1
Review the future contract for exact initial-button, one-append, fourth-generated-button, clear-document, finished-text, and white-border semantics before browser execution.

## Instructions

§ 5.1
Before any future browser run, review W01 and W02 as a bounded semantic flow. Check initial generated count 0, each of the five eligible transitions, current-last-only authority, one append per pre-completion press, vertical successor placement, no append on generated button 4, whole-document clearing, exact finished text, and rendered white-border requirement. Record pass or concrete mismatch without changing implementation.

## Acceptance criteria

§ 6.1
The review has an explicit pass/fail result for every listed invariant and confirms generated button 4 is the fourth appended button rather than the fourth total button. Any mismatch fails W07 and is returned to the owning unit; W07 itself makes no fix.

## Handoff

§ 7.1
A passing W07 authorizes W03 to execute exactly US-01. A failure returns to W01 or W02 and requires plan review if scope changes.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
