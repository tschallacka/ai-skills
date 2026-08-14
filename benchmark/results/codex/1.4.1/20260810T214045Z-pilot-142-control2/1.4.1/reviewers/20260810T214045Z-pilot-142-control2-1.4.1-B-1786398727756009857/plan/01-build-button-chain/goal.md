# Goal: Build button-chain.html interaction

## Current state and prior-goal handoffs

§ 2.1
No button-chain.html file exists in this planning proof. The future executor starts from a blank new file target and the benchmark forbids this planning run from creating, opening, serving, or testing HTML.

## Outcome and definition of done

§ 3.1
Create button-chain.html with the required initial button, append-only-last-button behavior, fourth-generated-button terminal state, and visible white-bordered finished state. Definition of done: the file-level markup, script, and style targets are implemented and ready for verification.

## Why this goal is needed

§ 4.1
This goal creates the entire future user-facing behavior in one HTML file before any verification can run. It groups only the mutually necessary markup, logic, and style units for the first independently demonstrable implementation state.

## Scope

§ 5.1
In scope: the initial button markup, the event logic for current-last-button clicks, generated-button counting, terminal document clearing, and the finished border style.

§ 5.2
Out of scope: automated tests, browser execution, artifact auditing, extra styling beyond the visible white border, persistence, accessibility enhancements not needed for this minimal proof, and any file other than button-chain.html.

## Affected files, systems, data, and interfaces

§ 6.1
button-chain.html owns all future implementation targets: #button-chain-root for markup, appendNextButton(event) for behavior, and .finished-state for terminal styling.

## Dependencies and handoffs

§ 7.1
No prior goal is required. This goal hands button-chain.html to W04 only after W07 confirms the markup, logic, and style work units are ready.

## Implementation approach, risks, and edge cases

§ 8.1
Implement the HTML as a small self-contained document. Store the generated-button count in script state, append a single new button only when event.target is the current last button, and replace document.body content with a single finished-state element on the fourth generated-button click. Edge cases: clicking an older button must do nothing, rapid repeated clicks must not append more than one button per accepted click, and the terminal state must remove the chain rather than leaving hidden or stale buttons. Finish the goal with W07 build readiness review before handoff.

## Owned work units

§ 9.1
`W01` — Create the initial document structure with one button inside a stable root/container for the chain.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The future implementation changes observable HTML behavior and must be proven by downstream static and browser verification units W04 and W05. |

§ 9.2
`W02` — Add event logic that appends exactly one button below the current last button and clears the document when the fourth generated button is pressed.

§ 9.3
`W03` — Style the terminal finished state so the exact lowercase text finished has a visible white border.

§ 9.4
`W07` — Review the completed implementation work units W01-W03 for readiness before handing the file to the verification goal.

## Goal-size exception

§ 11.1
N/A: this goal owns three work units, within the 2-10 work-unit limit.
