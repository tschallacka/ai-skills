# Goal: Define the future button-chain implementation contract

## Current state and prior-goal handoffs

§ 2.1
This is the first goal. The source brief supplies the future behavior and the plan description fixes the assumed future target at repository-root button-chain.html. No implementation artifact exists and no prerequisite handoff is required.

## Outcome and definition of done

§ 3.1
A resumable executor has exact markup, behavior, boundary, and proof instructions for the future button-chain HTML file, with no implementation artifact created in this proof.

## Why this goal is needed

§ 4.1
An executor needs separate, reviewable ownership for the future DOM subtree and the behavior callback. This goal turns the prose task into atomic implementation instructions without executing them during the proof.

## Scope

§ 5.1
Include only the future button-chain.html #button-chain markup and appendButtonChain() behavior contract, including the initial button, generated-button count, clear operation, finished text, and white border.

§ 5.2
Exclude creation or modification of button-chain.html, browser execution, server startup, unrelated styling, external dependencies, and any changes to the source brief.

## Affected files, systems, data, and interfaces

§ 6.1
The future executor changes one file, button-chain.html: the #button-chain DOM subtree in W01 and appendButtonChain() in W02. W03 in the next goal consumes both targets for browser proof.

## Dependencies and handoffs

§ 7.1
W01 precedes W02. W02 hands the exact initial-button and generated-button semantics to W03; the next goal must not infer a different counting rule or completion presentation.

## Implementation approach, risks, and edge cases

§ 8.1
Implement the initial button inside #button-chain, attach behavior to the current last button, append one new button below it per pre-completion click, count generated buttons rather than total buttons, and on generated button four clear the document and render finished with a white border. Edge cases are accidental double activation, appending more than one button, counting the initial button as generated, and leaving stale buttons after completion.

## Owned work units

§ 9.1
W01 owns the future #button-chain markup; W02 owns appendButtonChain() and its current-last-button callback; W07 owns the independent contract review. Together they define and proof the implementation contract consumed by W03.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The goal owns future markup and behavior work units; its downstream browser verification must prove one-button append sequencing and the fourth-generated-button completion output. |

§ 9.2
`W02` — Implement the future button-chain behavior: each current last-button press appends exactly one button below it, and the fourth generated-button press clears the document and prints finished with a white border.

§ 9.3
`W07` — Review the future button-chain implementation contract for exact initial-button, one-append, fourth-generated-button, clear, finished, and white-border semantics before browser execution.

## Goal-size exception

§ 11.1
No goal-size exception: this goal has three atomic work units, within the 2–10 limit.
