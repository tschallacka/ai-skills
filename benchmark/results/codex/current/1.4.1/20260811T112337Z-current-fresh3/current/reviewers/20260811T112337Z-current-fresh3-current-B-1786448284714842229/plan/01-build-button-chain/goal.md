# Goal: Build button-chain.html behavior

## Current state and prior-goal handoffs

§ 2.1
No button-chain.html file exists in this planning proof, and none should be created until a future execution session starts. The task contract and work-unit inventory define the expected file, DOM, behavior, and styling.

## Outcome and definition of done

§ 3.1
Create the standalone HTML, styling, and JavaScript behavior so the initial button grows a last-button-only chain and the fourth generated button clears the document to finished with a visible white border.

## Why this goal is needed

§ 4.1
This goal creates the complete future user-facing behavior and owns the primary browser story that proves it. Without these work units, there is no page, append logic, completion branch, visible white-bordered finished state, or direct-click evidence.

## Scope

§ 5.1
In scope: one standalone HTML file, one initial button, generated-button append behavior, last-button-only click handling, fourth-generated completion, and completion styling. Out of scope: tests, browser execution, additional files, frameworks, persistence, server routing, or non-button UI.

## Affected files, systems, data, and interfaces

§ 6.1
Future execution changes button-chain.html only, split across #button-chain-root, appendGeneratedButton(), handleButtonClick() last-button guard, handleButtonClick() fourth-generated completion branch, and .completion-state.

## Dependencies and handoffs

§ 7.1
W01 enables W02 by providing the root container. W02 enables W03 by defining append behavior. W03 enables W04 by centralizing click routing. W04 renders the exact finished text and enables W05 border styling. W06 depends on W01 through W05 for the completion story, and W08 depends on W01 through W03 for the non-last inert story.

## Implementation approach, risks, and edge cases

§ 8.1
Use an explicit generated index for appended buttons so the initial button is not counted as generated. Ensure handleButtonClick() compares the clicked button with the current last button before acting. The fourth generated button must clear the body and render exact text finished rather than append; earlier generated buttons append exactly one successor, and earlier non-last buttons remain inert.

## Owned work units

§ 9.1
`W01` — Create the standalone document structure with one initial button and no generated buttons in the initial DOM.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The goal changes user-visible HTML behavior and owns browser verification work unit W06, with additional static audit coverage in W07. |

§ 9.2
`W02` — Append exactly one new generated button immediately below the current last button each time the append path runs.

§ 9.3
`W03` — Ignore non-last buttons and route only the current last button click to append or completion behavior.

§ 9.4
`W04` — When the clicked current last button is the fourth generated button, clear the document body instead of appending another button.

§ 9.5
.

§ 9.6
`W06` — Using direct clicks, verify the initial button, four generated-button progression, no extra append on completion, document clear, exact finished text, and visible white border.

§ 9.7
`W05` — Style the completion state with a visible white border while preserving the exact text supplied by the completion branch.

§ 9.8
`W08` — Using direct clicks, verify that clicking an earlier non-last button after generated buttons exist does not append a button, clear the document, or trigger finished.

## Goal-size exception

§ 11.1
Not applicable. This goal owns seven work units, W01 through W06 plus W08, within the 2-10 work-unit limit.
