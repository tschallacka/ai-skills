# Goal: Create button-chain document contract

## Current state and prior-goal handoffs

§ 2.1
No prior implementation exists in this proof. The future executor starts from no button-chain.html file and must create it from the planned contract.

## Outcome and definition of done

§ 3.1
button-chain.html is specified with the required initial button, append-only last-button behavior, fourth-generated-button terminal state, and future verification coverage.

## Why this goal is needed

§ 4.1
The visible behavior depends on small but distinct targets; separating them prevents a broad implementation step from hiding markup, logic, and style changes.

## Scope

§ 5.1
In scope: document root and initial button, last-button append logic, white-border completion style, and fourth-generated terminal rendering.

§ 5.2
Out of scope: browser execution in this benchmark and any file other than button-chain.html in the future task.

## Affected files, systems, data, and interfaces

§ 6.1
button-chain.html is the only future implementation file. The user-facing interface is direct button clicking and the terminal finished message.

## Dependencies and handoffs

§ 7.1
W01 enables W02 and W03; W02 and W03 enable W04; W04 enables W07 to call the terminal renderer; W06 verifies the integrated implementation contract; W05 receives that handoff for browser-story verification.

## Implementation approach, risks, and edge cases

§ 8.1
Use a counter for generated buttons and treat a click on the fourth generated current-last button as terminal. Ignore clicks on any non-last earlier button so only the current last button can append or complete.

§ 8.2
Edge cases: clicking the original button creates generated 1; generated 1 creates generated 2; generated 2 creates generated 3; generated 3 creates generated 4; clicking generated 4 clears all prior content before rendering finished.

## Owned work units

§ 9.1
`W01` — Create the document body with exactly one initial button and a root container for the button chain.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | This goal defines testable UI behavior; each implementation step has a testing companion and downstream browser story W05 verifies the integrated result. |

§ 9.2
`W02` — Add click handling so only pressing the current last button appends exactly one new button below it.

§ 9.3
`W03` — Define the completion presentation so the final lowercase text finished is visible with a white border.

§ 9.4
`W04` — Clear the document when the fourth generated button is pressed and render only the exact lowercase text finished using the completion style.

§ 9.5
`W06` — Future executor verifies W01-W04 together before UI story execution: one initial button, one append per current-last click, terminal fourth-generated behavior, and finished with visible white border.

§ 9.6
`W07` — Add the handler branch that recognizes a click on the fourth generated current-last button and delegates to renderFinishedState() instead of appending another button.

## Goal-size exception

§ 11.1
N/A; this goal owns six work units, within the 2-10 unit limit.
