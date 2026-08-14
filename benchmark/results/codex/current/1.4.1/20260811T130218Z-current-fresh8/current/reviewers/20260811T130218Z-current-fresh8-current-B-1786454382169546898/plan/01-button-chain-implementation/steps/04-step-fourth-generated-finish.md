# Step: 04-step-fourth-generated-finish

## Ownership

- Goal: `01-button-chain-implementation`
- Work unit: `W04`
- Type: `source`

## Change target

- File: `button-chain.html`
- Primary symbol or file scope: `finishOnFourthGeneratedButton`
- Subscope: `N/A`

## Objective

§ 4.1
When the fourth appended/generated button is clicked as the current last button, clear the document and render only the completion state with exact lowercase text finished.

## Instructions

§ 5.1
Implement finishOnFourthGeneratedButton as the completion branch for the fourth appended button when that button is clicked as the current last button.

§ 5.2
On completion, clear existing document content and render one completion state containing exact lowercase text finished with the .completion-message style.

§ 5.3
Do not count the initial button as generated; the completion trigger is generated button number four.

## Acceptance criteria

§ 6.1
After generated buttons one through four have been created, clicking generated button four clears previous buttons and content.

§ 6.2
The resulting document shows exact text finished in lowercase and has a visible white border.

§ 6.3
Completion does not occur before the fourth generated button is clicked.

## Handoff

§ 7.1
W05 can test the full behavior from initial state through final completion.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
