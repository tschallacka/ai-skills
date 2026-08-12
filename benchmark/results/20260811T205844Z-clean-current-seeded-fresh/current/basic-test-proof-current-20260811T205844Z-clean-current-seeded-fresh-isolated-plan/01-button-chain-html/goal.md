# Goal: Define button-chain HTML behavior

## Current state and prior-goal handoffs

§ 2.1
No button-chain.html exists in this proof. Goal 01 owns only planned future targets W01-W04 and does not create files during this benchmark run.

## Outcome and definition of done

§ 3.1
A future executor can create button-chain.html with the exact initial button, append-only last-button interaction, fourth-generated-button completion behavior, and visible bordered finished state.

## Why this goal is needed

§ 4.1
The future page behavior depends on a precise load state, visual stacking, append rule, and completion rule. Keeping these as separate work units prevents the off-by-one completion behavior from being hidden inside broad implementation wording.

## Scope

§ 5.1
In scope for future execution: one HTML document body subtree, one button style selector, appendNextButton(), and finishDocument(). Out of scope: tests, browser execution, extra controls, persistence, counters displayed to the user, or alternative completion text.

## Affected files, systems, data, and interfaces

§ 6.1
Future affected file is button-chain.html. The markup, style, append function, and finish function are independently reviewable targets in that single file.

## Dependencies and handoffs

§ 7.1
W01 has no dependency. W02 depends on W01. W03 depends on W01 and W02. W04 depends on W03. Goal 02 can rely on these contracts for proof planning.

## Implementation approach, risks, and edge cases

§ 8.1
Use a generated count that distinguishes generated buttons from the initial button. Only the last button receives active append behavior; older buttons must be inert for appending. The fourth generated button must be identifiable and must call finishDocument() when pressed.

## Owned work units

§ 9.1
`W01` — Create the initial body structure with exactly one visible initial button and no completion text at load.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The HTML behavior is user-visible and must be proven by downstream DOM and browser-story verification work units W05 and W06. |

§ 9.2
`W02` — Style generated chain buttons as visible block-level controls stacked below the previous button.

§ 9.3
`W03` — Implement the click handler so only the current last button appends exactly one new button below it.

§ 9.4
`W04` — Implement the fourth generated button completion path so activation clears the document and prints lowercase finished with a visible white border.

§ 9.5
`W07` — Review the completed W01-W04 implementation against the exact five-click generated-button sequence before handing off to formal proof.

## Goal-size exception

§ 11.1
Not applicable; this goal owns five work units, within the 2-10 work-unit limit.
