# Goal: Define the button-chain implementation contract

## Current state and prior-goal handoffs

§ 2.1
No implementation exists in this proof. The benchmark contract and tagged skill are the only confirmed inputs; there are no prior-goal handoffs.

## Outcome and definition of done

§ 3.1
The future executor can implement the single HTML file from four atomic work units covering initial markup, exact append behavior, completion clearing, and visible completion styling.

## Why this goal is needed

§ 4.1
This goal converts the requested interaction into reviewable future file targets so an executor can implement and verify each behavior without inventing additional scope.

## Scope

§ 5.1
Include only the future button-chain.html markup, button-generation interaction, fourth-generated-button completion branch, and completion-message styling. Exclude all actual HTML creation or execution during this proof.

## Affected files, systems, data, and interfaces

§ 6.1
One future file, button-chain.html, with the body subtree, appendButton(), handleButtonActivation() fourth-activation branch, and .completion-message token listed as independent change targets.

## Dependencies and handoffs

§ 7.1
W01 establishes the initial target for W02. W02 creates generated buttons 1–4 and hands their counter and current-last targets to W03. W03 owns pressing generated button 4 and hands the completion message contract to W04. W04 and all preceding units hand off to W05 for UI validation.

## Implementation approach, risks, and edge cases

§ 8.1
Keep the current last button as the only append trigger. The initial button is not generated; the first four appended buttons are generated buttons 1–4. Append one sibling below the current last button per click, then pressing generated button 4 clears the document. Treat exact lowercase text, visible white border, and the initial one-button count as acceptance-critical edge cases.

## Owned work units

§ 9.1
`W01` — Define the document shell and one initial button as the only initial interactive control.

§ 9.2
`W02` — Append exactly one new button immediately below the current last button after each eligible activation and preserve a last-button target.

§ 9.3
`W03` — Count generated activations, clear the document on the fourth generated button, and emit the completion message contract.

§ 9.4
`W04` — Define a visible white border around the exact lowercase finished completion text.

## Goal-size exception

§ 10.1
No exception: this goal owns four work units, within the allowed 2–10 range.
