# Step: 01-step-us-01-validation

## Ownership

- Goal: `02-button-chain-ui-validation`
- Work unit: `W05`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `US-01 button-chain browser flow`
- Subscope: `N/A`

## Objective

§ 4.1
Execute US-01 from a fresh browser context using five cached rendered-button mouse clicks: four append clicks followed by a click on generated button 4, and record each checkpoint.

## Instructions

§ 5.1
Execute US-01 from a fresh browser context using only the cached rendered-button mouse clicks and record the decisive final observation.

## Acceptance criteria

§ 6.1
The story passes only if the initial one-button state, generated-button counts 1–4 after clicks 1–4, and the fifth-click completion state with exact lowercase finished and a visible white border are all observed; otherwise record untested or a bug with evidence.

## Handoff

§ 7.1
The implementation plan receives a bounded browser handoff; this isolated proof records that execution was not performed.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
