# Testing companion: implement the button-chain contract

## Automated tests

No repository test runner applies to this standalone file. After the file is
created, inspect the DOM in a browser and use the browser acceptance flow in
`02-step-browser-acceptance-testing.md`. If a lightweight DOM test is added,
it must assert exact counts and exact lowercase terminal text without changing
the single-file scope.

## Pass/fail criteria

Pass only if the implementation exposes one initial button, appends one and
only one button per qualifying click, and defines the fourth-generated-button
terminal transition. Fail on off-by-one counting, duplicate append, stale
handlers, or any terminal button remaining.

## Planning-proof boundary

These checks are instructions for a future implementation session. They were
not run here, and no HTML was created or inspected in this proof.
