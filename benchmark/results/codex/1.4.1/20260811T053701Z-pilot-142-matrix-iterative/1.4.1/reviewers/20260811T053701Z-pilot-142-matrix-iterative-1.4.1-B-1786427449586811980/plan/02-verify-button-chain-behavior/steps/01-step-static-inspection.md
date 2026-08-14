# Step: 01-step-static-inspection

## Ownership

- Goal: `02-verify-button-chain-behavior`
- Work unit: `W04`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `static artifact inspection`
- Subscope: `N/A`

## Objective

§ 4.1
Inspect button-chain.html after implementation to confirm the required file exists, contains exactly one initial button in source markup, contains no unrelated files, and encodes the exact finished text.

## Instructions

§ 5.1
After goal 01 execution, inspect the workspace file list and button-chain.html source without running it. Confirm button-chain.html is the only newly created HTML artifact for the future task.

§ 5.2
Read the file source and verify one initial source button, generated-button logic, exact lowercase finished text, and a white visible border style for .completion-message.

## Acceptance criteria

§ 6.1
Inspection evidence records that button-chain.html exists and no extra .html or .htm task artifacts were created.

§ 6.2
Inspection evidence records exact lowercase finished and a white border style in the source contract.

§ 6.3
If inspection fails, mark the step failed and enter the bug feedback loop before browser validation.

## Handoff

§ 7.1
W05 may proceed only when the static contract is satisfied or a resolved bug has been retested.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
