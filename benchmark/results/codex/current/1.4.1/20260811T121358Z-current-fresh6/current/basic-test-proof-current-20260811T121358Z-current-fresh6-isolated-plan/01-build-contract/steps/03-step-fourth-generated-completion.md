# Step: 03-step-fourth-generated-completion

## Ownership

- Goal: `01-build-contract`
- Work unit: `W03`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `completeOnFourthGenerated`
- Subscope: `N/A`

## Objective

§ 4.1
Add the completion branch so pressing generated button four clears the document and renders the exact text finished.

## Instructions

§ 5.1
When executing later, implement completeOnFourthGenerated so the fourth generated button is the completion trigger.

§ 5.2
On that trigger, clear the existing document content before rendering the completion message with exact lowercase text finished.

## Acceptance criteria

§ 6.1
After generated button four is pressed, no chain buttons remain visible and the only completion text is exactly finished.

## Handoff

§ 7.1
W04 can target the rendered completion message through the .completion-message selector.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
