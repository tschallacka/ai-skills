# UI user story: button-chain completion

As a visitor, I want to press the currently last button and receive one new button below it so that I can progress through a predictable button chain; after pressing the fourth generated button, I want the document to finish with the exact lowercase message `finished` in a visibly white-bordered completion state.

## Acceptance examples

- Given the page is loaded, when no action has occurred, then exactly one initial button is visible.
- Given the current last button, when it is pressed once, then exactly one button is appended below it.
- Given the first three generated buttons have been pressed in sequence, then the fourth generated button press clears the document.
- Given completion, then no buttons remain and the visible completion text is exactly `finished` with a visible white border.

## Out of scope

Keyboard/accessibility enhancements beyond normal button activation, persistence, responsive design, and server hosting are not acceptance requirements for this proof.
