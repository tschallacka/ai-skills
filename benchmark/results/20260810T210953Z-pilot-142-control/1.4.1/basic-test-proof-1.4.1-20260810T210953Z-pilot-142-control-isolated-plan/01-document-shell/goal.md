# Goal: Create button-chain document shell

## Current state and prior-goal handoffs

§ 2.1
No prior implementation goal exists. The future executor starts from a workspace where `button-chain.html` does not yet exist and must create only that file for the product artifact.

## Outcome and definition of done

§ 3.1
`button-chain.html` has one initial button, stable root markup, visible terminal-state styling, vertical chain-button layout, and goal-local source-inspection proof.

## Why this goal is needed

§ 4.1
This goal creates the visible foundation that the later chain handler and browser story depend on: a single initial button, a stable root for appended generated buttons, a terminal style selector, and a chain-button style that makes appended buttons appear below the previous current-last button.

## Scope

§ 5.1
Included: future creation of the `#button-chain-root` subtree, `.completion-state` selector, `.chain-button` vertical stack selector, and document-shell source inspection. Excluded: JavaScript chain behavior, browser execution, and any additional HTML file.

## Affected files, systems, data, and interfaces

§ 6.1
Affected future file: `button-chain.html`. Affected targets: `#button-chain-root` markup, `.completion-state` styling, `.chain-button` vertical stack styling, and the document shell source inspection proof.

## Dependencies and handoffs

§ 7.1
Prerequisite: none. Handoff to `02-chain-behavior`: the root subtree exists, contains exactly one initial button, exposes stable selectors for delegated click handling and completion rendering, and styles chain buttons so appended buttons appear below the previous current-last button.

## Implementation approach, risks, and edge cases

§ 8.1
Create minimal semantic markup first, then add terminal border styling and chain-button vertical layout without implementing behavior. The main risk is accidentally adding extra controls or script behavior in this goal; those belong to `W03`.

## Owned work units

§ 9.1
`W01` — Create the HTML document body with one initial button and a stable root for appended buttons.

§ 9.2
`W02` — Add visible completion styling with a white border around the finished state.

§ 9.3
`W06` — Style chain buttons so each generated button appears below the previous current-last button.

§ 9.4
`W07` — Verify the future document shell has one initial button, completion styling, and vertical chain-button layout before behavior work begins.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The markup and visible styling affect UI behavior and this goal owns a local verification work unit, W07. |

## Goal-size exception

§ 11.1
Not applicable. This goal owns four work units.
