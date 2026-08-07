# Goal: Define the future button-chain implementation contract

## Current state and prior-goal handoffs

§ 2.1
This first goal consumes only the exact brief and plan-level decisions. No implementation exists in this proof; the future target is repository-root button-chain.html.

## Outcome and definition of done

§ 3.1
A future executor can create the one-button initial DOM, implement deterministic current-last-button chaining, and check the semantics without editing or inferring any target outside W01, W02, and W07.

## Why this goal is needed

§ 4.1
Separate markup, executable behavior, and semantic proof ownership makes the one-file task independently reviewable while preventing counting or completion ambiguity.

## Scope

§ 5.1
Include only button-chain.html #button-chain markup, appendButtonChain(), its current-last-button callback, generated-button count, below-layout contract, clear-document completion, exact finished text, visible white border, and W07 semantic review.

§ 5.2
Exclude implementation during this proof, browser execution, server startup, external libraries, unrelated content or styling, persistence, reset behavior, and non-current-button activation behavior beyond ensuring old buttons cannot append again.

## Affected files, systems, data, and interfaces

§ 6.1
The future executor changes one file through two atomic scopes: #button-chain in W01 and appendButtonChain() with its current-last-button callback in W02. W07 is a no-file semantic verification flow.

## Dependencies and handoffs

§ 7.1
W01 precedes W02; W07 consumes W02. W07 hands an approved counting and completion contract to W03, which must not reinterpret the initial button as generated button 1.

## Implementation approach, risks, and edge cases

§ 8.1
W01 supplies one initial visible button and a vertical chain boundary. W02 assigns append authority only to the current last button, retires that authority when a successor is created, increments only newly appended buttons, appends exactly once for pre-completion presses, and replaces the whole document with completion output when generated button 4 is pressed.

§ 8.2
Review edge cases: rapid or double activation must not append twice from a retired button; clicking an earlier button must not append; generated button 4 must exist before its press completes; completion must append no fifth generated button; no stale button or prior node may survive; and the border must be observably white.

## Owned work units

§ 9.1
W01 owns the future #button-chain markup and initial state.

§ 9.2
W02 owns appendButtonChain() and the current-last-button callback behavior.

§ 9.3
W07 owns the bounded semantic contract review. Together they define the independently demonstrable future implementation contract consumed by Goal 2.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | Future markup and behavior are observable; W07 reviews the semantic contract and W03 later proves it in a real browser. |

§ 9.2
`W02` — Implement the future behavior: pressing the current last button appends exactly one button below it; pressing the fourth generated button clears the document and prints finished with a white border.

§ 9.3
`W07` — Review the future contract for exact initial-button, one-append, fourth-generated-button, clear-document, finished-text, and white-border semantics before browser execution.

## Goal-size exception

§ 11.1
No exception: this goal has three atomic work units, within the 2-10 limit.
