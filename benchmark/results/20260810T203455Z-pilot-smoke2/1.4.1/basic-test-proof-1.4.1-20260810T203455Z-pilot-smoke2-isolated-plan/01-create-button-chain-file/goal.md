# Goal: Create button-chain.html behavior

## Current state and prior-goal handoffs

§ 2.1
No prior implementation goal exists. The executor starts from no button-chain.html file and must create it as a future implementation step, not during this planning-only proof.

## Outcome and definition of done

§ 3.1
Create button-chain.html with one initial button, append-only last-button chaining, fourth-generated-button terminal clearing, and the exact finished state.

## Why this goal is needed

§ 4.1
This goal creates the entire user-visible behavior contract that the later browser story verifies.

## Scope

§ 5.1
Included: one initial button, append-only last-button click logic, generated-button counting, terminal clear behavior, exact finished text, and white border styling.

§ 5.2
Excluded: automated test file creation, browser execution, unrelated page content, persistence, frameworks, or server dependencies.

## Affected files, systems, data, and interfaces

§ 6.1
Future file: button-chain.html. Interface: direct browser clicks on visible buttons and visible document body output.

## Dependencies and handoffs

§ 7.1
W01 enables W02; W02 enables W03; W03 enables W04; W04 enables W06. When W06 passes, W05 can rely on an implemented standalone HTML file ready for direct browser verification.

## Implementation approach, risks, and edge cases

§ 8.1
Use a single source of truth for generated-button count and current-last-button eligibility. Ignore clicks on older generated buttons except the terminal click on generated button four if it is the current last button at that point.

§ 8.2
After terminal state, remove prior controls so the document contains only the completion message; keep the exact text separate from styling so the border cannot alter the string assertion.

## Owned work units

§ 9.1
`W01` — Create a valid HTML document containing exactly one initial button and no pre-rendered generated buttons.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The goal changes user-facing HTML behavior and must be verified by downstream UI story W05. |

§ 9.2
`W02` — Add click behavior so only pressing the current last button appends exactly one new button directly below it.

§ 9.3
`W03` — When the fourth generated button is pressed, clear the document and render only the completion state with exact text finished.

§ 9.4
`W04` — Style the completion state with a visible white border while preserving the exact lowercase text finished.

§ 9.5
`W06` — After W01-W04 are implemented in the future, review the file-level behavior contract before handing off to browser UI story validation.

## Goal-size exception

§ 11.1
Not applicable; this goal owns five work units, within the 2-10 work-unit limit.
