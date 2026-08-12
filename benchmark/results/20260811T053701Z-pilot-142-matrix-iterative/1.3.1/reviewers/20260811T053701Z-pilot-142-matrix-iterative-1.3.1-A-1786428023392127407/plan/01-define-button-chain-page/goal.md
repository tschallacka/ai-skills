# Goal: Define button-chain.html behavior

## Current state and prior-goal handoffs

§ 2.1
No implementation file exists in this planning-only proof. The future executor starts from the acceptance contract in plan-description.md and the work-unit inventory.

## Outcome and definition of done

§ 3.1
button-chain.html is planned as a single-page UI with one initial button, deterministic append behavior from the current last button, and a fourth-generated-button completion state that clears the document and shows finished with a visible white border.

## Why this goal is needed

§ 4.1
This goal establishes the complete future UI behavior and visual completion contract before any verification goal can run.

## Scope

§ 5.1
In scope are the future button-chain.html document body, append handler, finish handler, and completion border style. Tests and browser execution are owned by goal 02.

## Affected files, systems, data, and interfaces

§ 6.1
Affected future target is button-chain.html, split into the body subtree, appendNextButton(), finishOnFourthGeneratedButton(), and .completion-message scopes.

## Dependencies and handoffs

§ 7.1
This goal has no prior goal dependency. It hands goal 02 a complete future implementation target ready for DOM and browser verification.

## Implementation approach, risks, and edge cases

§ 8.1
The future implementation should track generated-button order, treat only the current last button as append-capable, and make the fourth generated button clear the document before rendering the completion message.

## Owned work units

§ 9.1
`W01` — Create the initial document body with exactly one starting button and a vertical container for generated buttons.

§ 9.2
`W02` — Define click handling so only pressing the current last button appends exactly one new button below it.

§ 9.3
`W03` — Define the fourth generated button click path so it clears the document and renders exact lowercase text finished.

§ 9.4
`W04` — Style the completion state so the finished text has a visible white border.

## Goal-size exception

§ 10.1
Not applicable. This goal owns four work units, within the 2 to 10 work-unit limit.
