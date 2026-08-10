# Goal: Define the button-chain HTML implementation

## Current state and prior-goal handoffs

§ 2.1
No prerequisite implementation exists in this isolated workspace. The tagged task contract is the sole prior input; the downstream browser goal receives the named file and selectors as its handoff.

## Outcome and definition of done

§ 3.1
Produce the complete future HTML implementation contract: initial button, append-on-last-button behavior through generated buttons 1–4, and terminal clearing when generated button 4 is pressed, followed by bordered lowercase finished output. Definition of done is a self-contained implementation plan with atomic markup, style, behavior, and contract-review units plus downstream browser proof ownership.

## Why this goal is needed

§ 4.1
This goal converts the user-visible contract into independently reviewable future file targets so an executor can implement without inferring hidden behavior.

## Scope

§ 5.1
In scope are the initial button subtree, the completion-message visual rule, the single initializer symbol W06, and the single click-handler symbol W03. Out of scope are any additional pages, frameworks, network requests, persistence, accessibility redesign beyond the named control semantics, and actual implementation during this proof.

## Affected files, systems, data, and interfaces

§ 6.1
button-chain.html only: #button-chain-root markup, .completion-message style, the button-chain initializer, and the button-chain click handler. The browser is not started by this proof.

## Dependencies and handoffs

§ 7.1
W01 establishes the control target for W06. W02 provides the terminal visual hook consumed by W03. W06 establishes the initializer/state consumed by W03. W03 hands the assembled future contract to W05; W05 gates W04, the required downstream browser proof.

## Implementation approach, risks, and edge cases

§ 8.1
Keep generated-button counting explicit: the initial button is not generated; pressing it appends generated button 1, pressing generated buttons 1–3 appends generated buttons 2–4, and pressing generated button 4 clears the document. The style hook must survive the clear-and-render operation and remain distinguishable against the terminal element's contrasting background.

## Owned work units

§ 9.1
`W01` — Define the single initial button and its containing DOM subtree in the future button-chain.html document.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The future HTML interaction and visible terminal state are user-facing; the goal's implementation units require downstream browser proof W04. |

§ 9.2
`W02` — Define the visible white border and presentation for the terminal finished message.

§ 9.3
`W03` — Implement the future click behavior: append exactly one button below the current last button through generated buttons 1–4, then clear the document when generated button 4 is pressed and render exact lowercase finished text using the completion-message styling hook.

§ 9.4
`W05` — Check that the three implementation units specify one concrete HTML file, one initial button, current-last traversal, exact append placement/counts, terminal DOM invariant, and explicit white-border declaration before handing off to browser validation.

§ 9.5
`W06` — Initialize #button-chain-root with one initial button, the generated-button counter state, and the current-last click-handler attachment point without implementing the click branch.

## Goal-size exception

§ 11.1
No exception: this goal owns four atomic work units, including W05 contract verification, and each has a separate step and testing companion.
