# Goal: Define button-chain HTML behavior

## Current state and prior-goal handoffs

§ 2.1
No implementation file is created in this proof. The future executor starts from an absent or empty button-chain.html target and must create only that file.

## Outcome and definition of done

§ 3.1
The future executor can create button-chain.html with one initial button, append exactly one button below the current last button on each last-button press, and render only the lowercase finished completion state with a visible white border when the fourth generated button is pressed.

## Why this goal is needed

§ 4.1
This goal defines the exact future UI behavior before browser verification so the executor cannot hide markup, behavior, and styling decisions inside one broad step.

## Scope

§ 5.1
Included: initial document subtree, append behavior, terminal completion behavior, and visible white border styling. Excluded: tests, browser execution, external assets, frameworks, storage, network calls, and additional files.

## Affected files, systems, data, and interfaces

§ 6.1
Affected future file: button-chain.html. Affected scopes: #button-chain-root, appendNextButton(), finishOnFourthGeneratedButton(), and .completion-message.

## Dependencies and handoffs

§ 7.1
W01 enables W02; W02 enables W03; W03 enables W04. The downstream verification goal consumes W01 through W04 and must not modify them during verification.

## Implementation approach, risks, and edge cases

§ 8.1
Use plain HTML, CSS, and JavaScript in button-chain.html. Track generated-button count explicitly, append only when the clicked target is the current last button, and render completion by replacing body content with a single bordered completion element containing exactly finished.

## Owned work units

§ 9.1
`W01` — Create the root document body subtree containing exactly one initial button and no completion message at load.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The generated HTML behavior is user-visible and must be covered by downstream browser verification work units. |

§ 9.2
`W02` — Define the click handler behavior so only the current last button appends exactly one new button below itself.

§ 9.3
`W03` — Define the terminal branch so pressing the fourth generated button clears the document and prints exactly lowercase finished.

§ 9.4
`W04` — Style the completion text with a visible white border while keeping the exact text content finished.

§ 9.5
`W07` — Verify by bounded code review after future implementation that W01 through W04 are present and ready for browser story execution.

## Goal-size exception

§ 11.1
Not applicable; this goal owns four work units.
