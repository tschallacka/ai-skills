# Goal: Create button-chain.html

## Current state and prior-goal handoffs

§ 2.1
This is the first future execution goal. It starts from no button-chain.html artifact in the benchmark workspace because the current run is planning-only.

§ 2.2
The executor may rely on the plan-level contract that button-chain.html is the only implementation file to create.

## Outcome and definition of done

§ 3.1
Create the single HTML artifact with one initial button, last-button-only append behavior, fourth-generated-button terminal clearing behavior, and the finished state styling contract. Definition of done: the file-level plan names the exact markup, script, style, and handoff-inspection targets without requiring any additional implementation files.

## Why this goal is needed

§ 4.1
This goal creates the entire user-facing artifact needed by the requested task; later verification cannot run until the markup, script behavior, and terminal styling exist.

## Scope

§ 5.1
Included: one initial button in source markup, generated buttons appended below the current last button, generated-button count handling, document clearing at the terminal click, and styling for the finished message.

§ 5.2
Excluded: tests, browser execution, extra HTML files, external assets, frameworks, build tooling, server setup, persistence, and accessibility enhancements beyond clear button text unless needed to make the planned browser story targetable.

## Affected files, systems, data, and interfaces

§ 6.1
Only button-chain.html is changed by this goal. The planned internal targets are document body markup, button-chain script click handler, and .completion-message CSS.

## Dependencies and handoffs

§ 7.1
This goal has no prerequisite implementation goal. It hands button-chain.html to goal 02 for static inspection and browser-story verification.

§ 7.2
The handoff must state the exact labels or selectors used for the initial and generated buttons, and must state that generated button 4 is appended before it is clicked as the terminal trigger.

## Implementation approach, risks, and edge cases

§ 8.1
Implement minimal standalone HTML: body contains the initial button and script; script tracks generated button count and only handles clicks on the current last button; style defines a visible white border for the completion message.

§ 8.2
Edge cases: clicking an earlier non-last button must not append a new button; generated button 4 must be appended by clicking generated button 3 before generated button 4 can be clicked as the terminal trigger; the terminal state must clear prior buttons before rendering finished.

## Owned work units

§ 9.1
`W01` — #button-chain-root markup creates one initial visible button and no generated buttons in source.

§ 9.2
`W02` — button-chain script click handler appends exactly one button below the current last button and clears when generated button 4 is clicked.

§ 9.3
`W03` — .completion-message style makes the exact finished text visibly bordered in white.

§ 9.4
`W06` — goal 01 handoff inspection confirms the implementation contract before downstream verification. These four units share the outcome of a complete single-file implementation artifact with an internal proof handoff.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The implementation goal changes visible markup, script behavior, and styling; its work units require downstream static and browser verification in goal 02. |

§ 9.2
`W02` — Add JavaScript so only the current last button appends exactly one new button below itself, with generated-button counting tracked deterministically.

§ 9.3
`W03` — Add the completion-state styling rule that makes the lowercase finished text visibly bordered in white after document clearing.

§ 9.6
`W06` — Confirm the implementation handoff names the initial button target, generated button labeling/count rule, terminal trigger as the fourth generated button, and .completion-message styling before goal 02 begins.

## Goal-size exception

§ 11.1
Not applicable. This goal owns four work units, within the 2-10 work-unit limit.
