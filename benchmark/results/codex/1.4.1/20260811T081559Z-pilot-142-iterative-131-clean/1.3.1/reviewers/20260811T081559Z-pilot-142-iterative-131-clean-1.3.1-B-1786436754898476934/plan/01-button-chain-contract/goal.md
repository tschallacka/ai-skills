# Goal: Button chain implementation contract

## Current state and prior-goal handoffs

§ 2.1
No prior implementation goal exists. The future executor starts from the benchmark request and must create button-chain.html as a new single file; this proof did not create or inspect any HTML.

## Outcome and definition of done

§ 3.1
Future button-chain.html contains the initial button, atomic DOM/style/behavior targets, and completion renderer needed for the required interaction. Definition of done: every planned implementation work unit is executable without touching any unnamed file or symbol and has downstream UI verification.

## Why this goal is needed

§ 4.1
The requested behavior depends on a precise relationship between DOM structure, completion styling, last-button click logic, and finish rendering. Separating those targets prevents an executor from hiding extra behavior inside a broad implementation step.

## Scope

§ 5.1
This goal covers only future edits to button-chain.html for the named DOM subtree, style selector, click handler, and completion renderer. It excludes browser execution, automated test creation, framework installation, server setup, and any file outside button-chain.html.

## Affected files, systems, data, and interfaces

§ 6.1
Future affected areas are button-chain.html #button-chain-app, button-chain.html .completion-message, button-chain.html handleButtonClick(event), and button-chain.html renderFinishedState(). No data store, backend, routing, asset pipeline, or external service is affected.

## Dependencies and handoffs

§ 7.1
W01 has no implementation dependency. W02 depends on the completion element contract from W01. W03 depends on W01. W04 depends on W02 and W03. The downstream handoff to W05 is a complete local HTML file ready for browser clicks.

## Implementation approach, risks, and edge cases

§ 8.1
Use a generated-button count stored in the page script, append only when event.target is the current last button, and call renderFinishedState() instead of appending when that current last button represents generated button 4. Guard the off-by-one case by naming generated buttons distinctly from the initial button in code or state.

## Owned work units

§ 9.1
`W01` — Create the document subtree with one initial button and no generated buttons present at load.

§ 9.2
`W02` — Define the completion message presentation with a visible white border.

§ 9.3
`W03` — Append exactly one new button below the current last button when that current last button is pressed, ignoring older buttons for append behavior.

§ 9.4
`W04` — When the fourth generated button is pressed, clear the document and render only the exact lowercase text finished using the completion-message styling.

## Goal-size exception

§ 10.1
Not applicable. This goal owns four mutually necessary implementation work units, within the 2 to 10 work-unit limit.
