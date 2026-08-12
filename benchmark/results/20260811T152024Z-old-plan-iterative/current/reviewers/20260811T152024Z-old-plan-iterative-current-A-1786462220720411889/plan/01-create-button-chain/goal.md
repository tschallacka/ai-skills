# Goal: Create button-chain HTML

## Current state and prior-goal handoffs

§ 2.1
 No button-chain.html exists in the benchmark workspace because this is a planning-only proof. Future execution starts from the repository/workspace state available at that time and creates exactly one new file.

## Outcome and definition of done

§ 3.1
button-chain.html is planned with independently reviewable markup, behavior, and styling work units that satisfy the requested interaction contract.

## Why this goal is needed

§ 4.1
 This goal converts the requested behavior into the single future HTML artifact while keeping markup, behavior, completion logic, style, and implementation review independently reviewable.

## Scope

§ 5.1
 In scope for future execution: create button-chain.html, define the initial button subtree, append only one button below the current last button per valid click, clear on the fourth generated button, and render the bordered finished state.

§ 5.2
 Out of scope: additional files, build tooling, frameworks, server routes, persistence, animations, browser testing during this proof, and any text other than exact lowercase finished in the completion state.

## Affected files, systems, data, and interfaces

§ 6.1
 Future file button-chain.html owns the #button-chain-root markup target, the button-chain script append and completion branches, and the .completion-message style selector.

## Dependencies and handoffs

§ 7.1
 W01 has no dependency. W02 depends on W01, W03 depends on W02, W04 depends on W03, and W06 verifies W01 through W04 before handing off to W05 in goal 02.

## Implementation approach, risks, and edge cases

§ 8.1
 Future implementation should keep a generatedCount state and a reference to the current last button. A click on the current last button appends one new button, increments generatedCount for generated buttons, and updates the last-button reference.

§ 8.2
 The completion branch must trigger when generated button 4 is clicked, replace document body content, and render a completion element whose textContent is exactly finished and whose white border is visually apparent.

## Owned work units

§ 9.1
`W01` — Create the minimal HTML document body with exactly one initial button and no pre-rendered generated buttons.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The goal changes observable UI behavior and must be verified by the downstream browser story work unit. |

§ 9.2
`W02` — Attach click handling so pressing only the current last button appends exactly one new button below it.

§ 9.3
`W03` — When the fourth generated button is pressed, clear the document and render only the completion state.

§ 9.4
`W04` — Style the completion state so the exact lowercase text finished has a visible white border.

§ 9.5
`W06` — Review the completed button-chain.html against W01 through W04 before running browser story US-01.

## Goal-size exception

§ 11.1
Not applicable. This goal owns five work units, within the 2-10 work-unit limit.
