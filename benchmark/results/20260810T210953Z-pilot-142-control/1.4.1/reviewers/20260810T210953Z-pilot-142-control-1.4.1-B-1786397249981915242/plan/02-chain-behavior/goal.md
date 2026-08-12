# Goal: Add chain interaction behavior

## Current state and prior-goal handoffs

§ 2.1
This goal depends on `01-document-shell`: `button-chain.html` must contain the root subtree, exactly one initial button, completion styling, and button-stack layout before behavior is added.

## Outcome and definition of done

§ 3.1
`button-chain.html` appends exactly one button when the current last button is pressed, rejects non-last-button clicks, and reaches the finished state only from the fourth generated button.

## Why this goal is needed

§ 4.1
This goal owns the core interaction contract. Without it, the page cannot distinguish the current last button, generated-button count, or terminal click behavior.

## Scope

§ 5.1
Included: future implementation of `handleChainClick(event)` in `button-chain.html` and source-inspection proof of its behavior. Excluded: changing markup outside the established root, changing completion styling, and running the final browser story.

## Affected files, systems, data, and interfaces

§ 6.1
Affected future file: `button-chain.html`. Affected target: `handleChainClick(event)` and its local state for generated button count and terminal rendering.

## Dependencies and handoffs

§ 7.1
Prerequisite: `W07` complete. Handoff to `03-verification-handoff`: clicking the current last button appends one button below it, non-last buttons do not append, and clicking the fourth generated button replaces the document with bordered `finished` text.

## Implementation approach, risks, and edge cases

§ 8.1
Use event delegation from the root or a single click binding that rejects clicks not targeting the current last button. Track generated buttons separately from the initial button so the terminal state triggers on the fourth generated button.

## Owned work units

§ 9.1
`W03` — Implement delegated click handling so only the current last button appends one button and the fourth generated button clears the document to finished.

§ 9.2
`W08` — Verify the future click handler accepts only the current last button and treats the fourth generated button as terminal before final browser proof.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The click behavior is the core observable interaction and this goal owns a local verification work unit, W08. |

## Goal-size exception

§ 11.1
Not applicable. This goal owns one behavior work unit and one local verification work unit.
