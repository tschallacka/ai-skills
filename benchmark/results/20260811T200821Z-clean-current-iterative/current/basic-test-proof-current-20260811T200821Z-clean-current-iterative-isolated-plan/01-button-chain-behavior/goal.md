# Goal: Create button chain behavior

## Current state and prior-goal handoffs

§ 2.1
No implementation file exists in this proof. Future execution begins by creating button-chain.html from the task contract only.

## Outcome and definition of done

§ 3.1
button-chain.html can be created with the initial button, current-last-button append rule, fourth-generated-button clearing rule, and an automated behavior test design.

## Why this goal is needed

§ 4.1
This goal owns the core behavior: the initial button, the exact append rule, the fourth-generated completion branch, and behavior-level test design.

## Scope

§ 5.1
In scope: one HTML file, one initial button, generated-button counting, last-button-only clicks, completion text, and automated behavior checks. Out of scope: CSS border styling and browser-story execution, which are owned by goal 02.

## Affected files, systems, data, and interfaces

§ 6.1
button-chain.html scopes #button-chain-root, appendNextButton, finishOnFourthGeneratedButton, and buttonChainBehaviorTest.

## Dependencies and handoffs

§ 7.1
No prior goal is required. This goal hands W03 completion state and W04 behavior proof design to goal 02.

## Implementation approach, risks, and edge cases

§ 8.1
Track generated buttons separately from the initial button so the fourth generated button is the fourth appended button. The click handler must reject clicks on any button that is not currently last.

## Owned work units

§ 9.1
`W01` — Create the initial document subtree with one initial button and a container for appended buttons.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | This goal defines observable DOM behavior and owns an automated test-design work unit. |

§ 9.2
`W02` — Add the click handler behavior that appends exactly one button below the current last button only when that current last button is pressed.

§ 9.3
`W03` — Add the branch that clears the document when the fourth generated button is pressed and prints exactly finished.

§ 9.4
`W04` — Define an automated test target that exercises initial state, exact single append per current-last click, ignored non-last clicks, and fourth-generated completion text.

## Goal-size exception

§ 11.1
Not applicable; this goal owns four work units, within the 2-10 work-unit limit.
