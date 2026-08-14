# Goal: Create button-chain.html behavior

## Current state and prior-goal handoffs

§ 2.1
There are no prior goals. The future file button-chain.html is not present in this proof workspace and must be created only when the implementation plan is executed later.

## Outcome and definition of done

§ 3.1
A future executor can create button-chain.html with one initial button, append exactly one button below the current last button per valid click, and replace the document with the exact lowercase text finished inside a visible white border when the fourth generated button is pressed.

## Why this goal is needed

§ 4.1
This goal captures the complete future behavior requested by the benchmark in one independently demonstrable outcome: the initial button chain, exact append semantics, terminal clear, exact finished text, visible white border, and browser proof.

## Scope

§ 5.1
In scope for execution: create a single self-contained button-chain.html file, implement the button-chain interaction, style the completion state, and verify the story through real browser clicks.

§ 5.2
Out of scope: additional HTML files, scripts, dependencies, build systems, storage, timers, keyboard shortcuts, and benchmark-time HTML/browser execution.

## Affected files, systems, data, and interfaces

§ 6.1
The only future implementation file is button-chain.html. It contains markup under #button-chain-root, JavaScript functions appendNextButton() and finishOnFourthGeneratedButton(), and CSS selector .completion-message. The only user-facing interface is the rendered browser page and its visible button clicks; no backend, database, API, or network system is involved.

## Dependencies and handoffs

§ 7.1
W01 must complete before behavior work can attach to the initial/root elements. W02 depends on W01. W03 depends on W02's generated-button state. W04 depends on W03's completion markup. W05 depends on all implementation and style units.

§ 7.2
The goal handoff to a future executor is the exact acceptance contract: after the terminal click, the document contains only the bordered finished state, not the prior buttons.

## Implementation approach, risks, and edge cases

§ 8.1
Use an explicit generated count stored in script state or data attributes. The initial button starts the chain but is not itself a generated button for the terminal count.

§ 8.2
When a button is clicked, first confirm it is the current last button in the chain. If it is not last, do nothing so earlier buttons cannot add duplicates or violate exactly-one append behavior.

§ 8.3
When generated button 4 is clicked, replace the document body or root contents with a single completion element whose textContent is exactly finished and whose border is visibly white against a contrasting background.

## Owned work units

§ 9.1
W01 through W05 are owned by this goal. They are grouped because none is independently useful without the same button-chain acceptance contract, and together they stay within the 2-10 work-unit goal size limit.

§ 9.2
W01 owns markup structure; W02 owns exactly-one append behavior; W03 owns terminal clearing and exact text; W04 owns visible border styling; W05 owns future browser story verification.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The goal changes observable HTML UI behavior, so future browser verification is required; this benchmark records the verification plan but does not execute it. |

§ 9.2
`W02` — Implement the click path for the current last button so one and only one new button is appended below it.

§ 9.3
`W03` — Implement the terminal branch so pressing the fourth generated button clears the document and renders exactly finished.

§ 9.4
`W04` — Style the completion state so the finished text has a visible white border.

§ 9.5
`W05` — Verify through real browser clicks that the button chain appends exactly one button per click and completes with bordered finished text after the fourth generated button is pressed.

## Goal-size exception

§ 11.1
Not applicable. This goal owns five work units, within the required 2-10 range.
