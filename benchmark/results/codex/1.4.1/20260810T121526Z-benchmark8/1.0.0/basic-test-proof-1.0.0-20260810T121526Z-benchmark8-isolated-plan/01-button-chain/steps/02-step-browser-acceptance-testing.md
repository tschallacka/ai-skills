# Testing companion: browser acceptance

## Browser verification

Use a fresh browser page for `button-chain.html`. Observe initial button count
1. Click the current last button for generated buttons 1 through 3 and expect
total counts 2, 3, and 4. Click generated button 4 and expect zero buttons,
exact text `finished`, and a visible white border around the completion
element. Any click on a non-last button before completion must leave the count
unchanged.

## Pass/fail criteria

Pass requires all counts, ordering, exact spelling/case, and visible border.
Fail if any click appends zero or more than one button, if an old button
remains, if the fourth overall rather than fourth generated button triggers,
or if the border is not visibly white.

## Planning-proof boundary

The browser flow is deliberately not executed in this run. Starting a browser,
server, driver, or other execution tooling is prohibited by the proof task.
