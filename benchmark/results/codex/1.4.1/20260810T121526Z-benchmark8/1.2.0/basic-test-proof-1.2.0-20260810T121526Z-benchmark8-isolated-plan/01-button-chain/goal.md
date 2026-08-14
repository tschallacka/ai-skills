# Goal 01: button-chain

## Current state and prior-goal handoffs

No prior goal exists. The plan is a draft for a future standalone HTML implementation; the proof itself has not created or run HTML.

## Outcome and definition of done

Define an executable, independently reviewable implementation contract and static safety handoff for the button-chain. Done when the future executor can create one initial button, append exactly one below the current last button per click, clear on generated `Button 4`, show exact `finished` with a visible white border, and the tagged validator and safety audits are separately specified.

## Why this goal is needed

It owns the single coherent user-visible outcome required by the benchmark and gives a later executor a complete handoff without reconstructing behavior from prose.

## Scope

In scope are the one HTML file's named markup, behavior functions, completion style, tagged validation, HTML artifact audit, and process cleanup audit. Browser story execution and result/report artifacts are owned by final goal 02. Out of scope are all other files, frameworks, persistence, networking, and present-day HTML execution.

## Affected files, systems, data, and interfaces

Future file: `button-chain.html`. Verification interface: a local browser route. Plan interface: the Markdown artifacts in this directory.

## Dependencies and handoffs

W01 precedes W02, W03, and W04. W06, W07, W09, and W10 provide static handoff evidence; the final UI-validation goal consumes the implementation and audit handoffs. No external dependency is assumed.

## Implementation approach, risks, and edge cases

Use a stable container and explicit last-button reference. Label the initial button `Button 0` and generated buttons `Button 1` through `Button 4` so browser targets are deterministic and accessible. Each click on the current last button must add one element only; after clicks on buttons 0, 1, 2, and 3 have created generated buttons 1–4, pressing generated button 4 is the terminal branch and must clear prior content before adding the completion message. Verify no fifth button is added, text is lowercase and exact, and the border is visibly white.

## Owned work units

W01–W04, W06, W07, W09, and W10 are owned by this goal. Together they cover implementation plus separate validator, report, HTML-artifact, and process-cleanup targets.
