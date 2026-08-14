# Goal: button-chain behavior

## Current state and handoff

No HTML exists as a result of this proof. The shared contract is in the plan description. This goal owns the future standalone implementation and its acceptance evidence.

## Outcome and definition of done

Create `button-chain.html` with one initial button. Each press of the current last button appends exactly one button directly below it. Pressing the fourth appended button clears the document and leaves the exact lowercase text `finished` with a visible white border. Done requires the browser sequence and static checks in the testing companion to pass.

## Why this is needed

This is the complete user-visible behavior required by the future task and supplies the implementation handoff from the planning proof.

## Scope

In scope are the single file, its button state/count, event handling, append placement, completion transition, and required completion styling. Out of scope are frameworks, persistence, backend behavior, and unrelated visual design.

## Affected files and interfaces

Future file: `button-chain.html`. Interface: the document body and visible buttons. No external interface is required.

## Dependencies and handoff

No prior goal is required. After implementation, hand off the resulting file and the observed sequence to the testing companion and acceptance reviewer.

## Implementation approach, risks, and edge cases

Track appended-button count separately from the initial button. Bind behavior so only the current last button is actionable. On ordinary presses, append one new button below the current last button and make the new last button the next target. On the fourth appended-button press, replace/clear the document with only `finished` and apply a nonzero white border. Check that a press does not append two buttons, that the initial button is not counted as generated, and that the completion state cannot continue appending.
