# Plan: Basic test proof 1.4.1 isolated: button-chain implementation plan

## Current state

§ 2.1
The source brief requires a planning-only proof using repository revision 1.4.1. The current planning skill, mandatory UI-validation reference, and bundled helpers are available; .codegraph is absent. Pre-existing repository files are preserved, and no HTML artifact is inspected or changed.

## Desired outcome

§ 3.1
Produce a validated, resumable plan for future button-chain.html execution: exactly one initial button; pressing the current last button appends exactly one button below it; pressing the fourth newly generated button clears the document and prints finished with a visible white border. Planning artifacts, UI acceptance, review, trackers, and handoff are complete while implementation and browser work remain deferred.

## Approach

§ 4.1
Use the helper-created work-unit inventory as the scope source of truth; separate markup, behavior, semantic review, browser proof, story documentation, structural validation, and reporting into atomic units; record the direct-click story and user-approved exclusion; perform a disclosed sequential adversarial review because subagents are forbidden; approve, validate, create trackers, and report.

## Scope

§ 5.1
In scope: the future button-chain.html contract, #button-chain markup, appendButtonChain() behavior, fourth-generated-button completion state, bounded browser acceptance instructions, UI artifacts, adversarial review, structural validation, progress tracking, and execution report.

§ 5.2
Out of scope for this proof: creating, editing, opening, serving, or testing HTML; reading an existing HTML file; starting a browser, server, or driver; executing the future task; spawning workers; changing planning helpers; or modifying any pre-existing file.

## Affected areas

§ 6.1
Future implementation target: repository-root button-chain.html, with the named #button-chain DOM subtree and appendButtonChain() behavior scope. The brief explicitly supplies this filename.

§ 6.2
Current changes are confined to basic-test-proof-1.4.1-isolated-plan: its plan description, inventory, goals, atomic steps and companions, UI story artifacts, review, progress trackers, context snapshot, and execution report.

## Constraints and decisions

§ 7.1
The user requires planning-only execution, exact directory isolation, sequential work, no subagents or parallel workers, no HTML access, no browser/server/driver startup, bundled helper use, complete decomposition, UI artifacts, review, trackers, validation, and a concise execution summary.

§ 7.2
The fourth generated button means the fourth button appended after the initial button. UI validation is required for future execution but explicitly excluded now by user approval. The independent-agent gate cannot be satisfied under the no-subagent constraint, so the review artifact discloses a separate sequential adversarial pass rather than claiming independent review.

## Risks and open questions

§ 8.1
No material scope question remains. Future execution must preserve the generated-button counting rule and must reopen review if the target, behavior, or user-approved browser exclusion changes.

§ 8.2
The present plan cannot provide rendered evidence by design; US-01 remains excluded until a future executor is authorized to create and open the HTML file.

## UI classification

- UI affected: yes
- Rationale: The future task creates a user-visible HTML interaction and terminal visual state, so UI validation is mandatory even though execution is explicitly deferred in this planning-only proof.

## UI validation

- Required: yes
- Browser target: Planning-only exclusion: future executor must open button-chain.html in a real rendered browser
- Story artifact: `ui-user-stories.md`

## Adversarial review

- Artifact: `adversarial-review.md`
- Status: ✅ approved
