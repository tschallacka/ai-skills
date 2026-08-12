# Goal: Implement and verify button chain

## Current state and prior-goal handoffs

§ 2.1
This is the first and only planned goal. No prerequisite implementation handoff exists. The benchmark has not created button-chain.html and forbids doing so during planning.

## Outcome and definition of done

§ 3.1
Define the future button-chain.html change and its proof so an executor can create one initial button, append exactly one button below the current last button on each valid press, and replace the document with lowercase finished inside a visible white border when the fourth generated button is pressed.

## Why this goal is needed

§ 4.1
This goal converts the requested UI behavior into atomic implementation and verification targets so a later executor can complete the HTML task without inferring missing files, symbols, or acceptance rules.

## Scope

§ 5.1
In scope for future execution: one new button-chain.html file, its initial markup, append handler, completion handler, completion styling, and the US-01 browser flow.

§ 5.2
Out of scope: any implementation during this benchmark, external assets, frameworks, persistence, server setup, browser automation in the proof run, and behavior for clicking non-last buttons beyond confirming they do not append.

## Affected files, systems, data, and interfaces

§ 6.1
Implementation target: button-chain.html. Markup target: #button-chain-root. Source targets: appendNextButton() and completeChain(). Style target: .completion-message. Verification target: US-01 browser flow.

## Dependencies and handoffs

§ 7.1
W01 must finish before W02. W02 must finish before W03. W03 must finish before W04. W05 depends on W01 through W04 and is the final proof handoff. After W05 passes in a future implementation run, the handoff should include the browser route, click sequence, observed button counts after each click, final text, and visual border evidence.

## Implementation approach, risks, and edge cases

§ 8.1
Use simple semantic HTML and plain JavaScript in button-chain.html. Maintain a generated-button count separate from the initial button, append one button only when the clicked element is the current last button, and route the fourth generated button click to completeChain(). Edge cases: older generated buttons are no longer the current last button, rapid repeated clicks must not append more than one button per accepted click, the fourth generated button clears all previous content, and the white border remains visible.

## Owned work units

§ 9.1
`W01` — Create the document body structure with one initial button and a stable container for generated buttons.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The goal changes observable UI behavior and therefore owns W05 browser verification plus step companions for each implementation target. |

§ 9.2
`W02` — Add the click handling that only responds when the clicked control is the current last button and appends exactly one new button below it.

§ 9.3
`W03` — Add the completion branch for the fourth generated button that clears the document and renders exactly finished.

§ 9.4
`W04` — Style the completion state so the lowercase finished text has a visible white border.

§ 9.5
`W05` — Future browser verification: click the current last button to create generated buttons one through four, then click the fourth generated button and confirm document clearing, exact finished text, and visible white border.

## Goal-size exception

§ 11.1
Not applicable: this goal owns five work units, within the 2-10 work-unit limit.
