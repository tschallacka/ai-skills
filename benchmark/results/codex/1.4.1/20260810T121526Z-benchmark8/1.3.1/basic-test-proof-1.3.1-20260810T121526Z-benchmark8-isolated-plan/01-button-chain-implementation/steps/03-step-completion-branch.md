# Step: 03-step-completion-branch

## Ownership

- Goal: `01-button-chain-implementation`
- Work unit: `W03`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `handleButtonActivation()`
- Subscope: `generated-button-4 completion branch`

## Objective

§ 4.1
Define the generated-button-4 branch in handleButtonActivation() so pressing generated button 4 clears the document and emits only the exact lowercase text finished.

## Instructions

§ 5.1
Define the generated-button-4 branch in handleButtonActivation() so pressing generated button 4 clears the document and emits only the exact lowercase text finished.

## Acceptance criteria

§ 6.1
After the fifth click, which presses generated button 4, no prior buttons remain and the completion text is exactly finished.

## Handoff

§ 7.1
W04 can style the emitted completion message through .completion-message; W05 can verify that the fifth click targets generated button 4.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
