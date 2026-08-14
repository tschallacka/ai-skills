# Goal: button-chain behavior plan

## Current state and handoff

No `button-chain.html` has been created in the proof workspace. The shared
contract is in the plan description. This goal owns the future single-file
implementation and its browser acceptance evidence.

## Outcome and definition of done

The future implementation provides one initial button; each click on the
current last button appends exactly one button below it; clicking generated
button 4 clears the document; and the resulting document visibly prints exact
lowercase `finished` inside an element with a visible white border. Done means
the implementation and its listed browser checks pass.

## Why this goal is needed

It converts the small interaction into a deterministic state machine that a
later agent can implement without reconstructing counting or terminal-state
semantics.

## Scope and affected interfaces

In scope: root-level `button-chain.html`, button DOM order, click handling,
generated-button count, terminal rendering, and browser verification. Out of
scope: server-side code, dependencies, persistence, and unrelated styling.

## Dependencies and handoff

No external dependency. The implementation step hands `button-chain.html` to
the browser-verification step. Verification hands a pass/fail sequence and
screenshots or DOM observations to the final reviewer.

## Implementation approach, risks, and edge cases

Start with one button and an explicit generated-count state of zero. On a
click, confirm the clicked element is the current last button. If it is the
fourth generated button, replace the document contents with the completion
element; otherwise increment the count and append exactly one button after the
current last button. Keep the handler attached to each new button. Test that a
non-last button does not append, that each qualifying click changes the count
by one, and that the terminal state has no buttons remaining.

## Handoff

Outcome: an implementation-ready state-machine contract is present in the
step, story, and testing companion files. Files to create later: only
`button-chain.html`. Verification: specified but intentionally not run during
this planning-only proof. Caveat: terminal styling must visibly use a white
border in the eventual browser.
