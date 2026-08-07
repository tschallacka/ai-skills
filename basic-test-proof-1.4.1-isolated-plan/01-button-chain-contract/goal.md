# Goal: Define the future button-chain implementation contract

## Current state and prior-goal handoffs

§ 2.1
This is the first goal. The brief fixes the future target as repository-root button-chain.html and supplies the exact interaction. No implementation artifact exists and no prerequisite handoff is required.

## Outcome and definition of done

§ 3.1
A future executor has exact, atomic markup, behavior, and semantic-review instructions for button-chain.html, while this planning proof creates or inspects no HTML.

## Why this goal is needed

§ 4.1
Separate ownership for the DOM subtree, callback, and semantic proof makes the small UI task independently reviewable and prevents ambiguity about which button triggers completion.

## Scope

§ 5.1
Include only the future button-chain.html #button-chain subtree, appendButtonChain() current-last-button callback, generated-button count, document clearing, finished text, white border, and contract review.

§ 5.2
Exclude HTML creation or access during this proof, browser execution, server startup, unrelated styling, dependencies, and changes to pre-existing files.

## Affected files, systems, data, and interfaces

§ 6.1
The future executor changes one file: button-chain.html. W01 owns #button-chain markup; W02 owns appendButtonChain() and its current-last-button callback; W07 owns a bounded semantic contract review with no file target.

## Dependencies and handoffs

§ 7.1
W01 precedes W02; W07 consumes W02. W07 hands the confirmed generated-button counting and completion semantics to W03 in Goal 2.

## Implementation approach, risks, and edge cases

§ 8.1
Start with exactly one visible initial button. Make only the current last button append exactly one button directly below itself. Count appended buttons separately from the initial button. Pressing appended button four clears the document and renders only finished inside a visible white border. Guard against duplicate activation, extra appends, incorrect counting, and stale buttons.

## Owned work units

§ 9.1
W01 owns the future #button-chain markup; W02 owns appendButtonChain() and its callback; W07 owns the future semantic contract review. Together they define one demonstrable button-chain contract.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The markup and behavior are observable UI work; W07 and downstream W03 provide semantic and rendered proof, deferred now only by the explicit planning-only boundary. |

§ 9.2
`W02` — Implement the future button-chain behavior: pressing the current last button appends exactly one button below it, and pressing the fourth generated button clears the document and prints finished with a white border.

§ 9.3
`W07` — Review the future button-chain contract for exact initial-button, one-append, fourth-generated-button, clear, finished, and white-border semantics before browser execution.

## Goal-size exception

§ 11.1
No exception is needed: this goal owns three atomic work units, within the 2–10 limit.
