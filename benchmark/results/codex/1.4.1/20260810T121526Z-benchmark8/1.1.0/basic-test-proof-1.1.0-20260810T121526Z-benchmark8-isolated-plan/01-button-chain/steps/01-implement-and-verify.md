# Step: implement and verify button chain

## Owning goal and objective

Owner: `01-button-chain`. Create the standalone HTML behavior and hand it to verification.

## Files or areas touched

Create and edit only future `button-chain.html`.

## Executable implementation instructions

1. Start with one visible initial button and an appended-button counter at zero.
2. On the current last button’s activation, increment the generated count. If it is below four, append exactly one new button immediately below the current last button and ensure the new button is the only next interaction target.
3. When the generated count reaches four, clear the document and render only the exact text `finished`; give that completion element a nonzero, visible white border.
4. Keep completion terminal: no button remains and no further append can occur.
5. Run the companion checks and record observed results before marking this step complete.

## Acceptance criteria

- Initial load has exactly one button.
- Each of the first three generated-button activations increases the total by exactly one and places the new button below the previous last button.
- The fourth generated-button activation clears the document.
- Completion visibly contains exactly `finished` and has a visible white border.
- No implementation or verification step relies on a server or external dependency.

## Handoff

Provide `button-chain.html`, the observed interaction sequence, and static evidence to the goal reviewer. This step remains incomplete in this planning-only proof.
