# Step: implement the button-chain contract

## Owning goal and objective

Goal `01-button-chain`; create the future `button-chain.html` according to the
exact button-count and terminal-state contract.

## Files or areas

Create only the root-level `button-chain.html`. No framework or external asset
is needed.

## Executable implementation instructions

1. Render one initial button and no generated buttons.
2. Maintain an explicit generated-button count initialized to zero.
3. On a click, act only when the clicked button is the current last button.
4. For generated counts below four, increment once and append exactly one new
   button below the current last button, with the same handler behavior.
5. When the clicked button is generated button four, clear the document and
   render exactly `finished` in lowercase with a visible white border.
6. Keep the terminal state free of buttons and do not append after completion.

## Acceptance criteria

The resulting file meets every step in the UI story and has no additional
framework, persistence, or network behavior.

## Later handoff

Give the created file to step 02 for browser verification. Report the final
button counts after each click and the terminal text/border observation.
