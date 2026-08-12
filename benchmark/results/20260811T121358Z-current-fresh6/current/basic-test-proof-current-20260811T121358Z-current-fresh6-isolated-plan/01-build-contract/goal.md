# Goal: Build the button chain contract

## Current state and prior-goal handoffs

§ 2.1
No implementation file exists in this proof. The executor starts from the future requirement and must create button-chain.html as a standalone document.

## Outcome and definition of done

§ 3.1
button-chain.html has the planned static structure, behavior logic, completion branch, completion styling, and a static acceptance review ready before browser validation.

## Why this goal is needed

§ 4.1
This goal owns the concrete file contract so the later UI story can verify behavior without adding unplanned implementation scope.

## Scope

§ 5.1
Included are the initial button subtree, delegated button click logic, fourth-generated completion behavior, white-border finished state, and static acceptance review. Excluded are browser execution and any second file.

## Affected files, systems, data, and interfaces

§ 6.1
Future affected file is button-chain.html. Planned atomic targets are #button-chain-root, handleButtonClick, completeOnFourthGenerated, .completion-message, and a source-review verification flow.

## Dependencies and handoffs

§ 7.1
This goal has no prerequisite goal. It hands button-chain.html and the static acceptance checklist to 02-validate-ui-story.

## Implementation approach, risks, and edge cases

§ 8.1
Use event delegation or equivalent guarded logic so only the current last button is active for appending. Count only generated buttons, not the initial button, and clear the document before rendering the finished message.

## Owned work units

§ 9.1
`W01` — Create the standalone document body subtree with exactly one initial button visible at load.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The goal changes user-visible markup, behavior, and style, so it owns a static verification work unit before final browser validation. |

§ 9.2
`W02` — Add the click handler that appends exactly one generated button below the current last button and ignores non-last buttons for append behavior.

§ 9.3
`W03` — Add the completion branch so pressing generated button four clears the document and renders the exact text finished.

§ 9.4
`W04` — Style the finished state so the text finished has a visible white border.

§ 9.5
`W05` — Review the future button-chain.html source against the initial button, append, completion, exact text, and white-border contract before browser validation.

## Goal-size exception

§ 11.1
Not required because this goal owns five work units.
