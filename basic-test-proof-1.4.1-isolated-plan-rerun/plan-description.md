# Plan: Basic test proof 1.4.1: isolated button-chain plan rerun

## Current state

§ 2.1
The exact brief requires a fresh isolated planning-only proof using repository planning/SKILL.md as revision 1.4.1. Prior 1.3.1 and 1.4.0 reports provide comparison shape only. No .codegraph directory is available, and this run has not created, edited, opened, served, or tested HTML or started browser, server, driver, subagent, or parallel worker processes.

## Desired outcome

§ 3.1
Produce a resumable plan for repository-root button-chain.html with one initial button; pressing the current last button appends exactly one button below it; pressing the fourth generated button clears the entire document and prints the exact lowercase text finished with a visible white border. The plan is complete when decomposition, UI artifacts, sequential adversarial review, trackers, validation, bounded context checks, and the 1.4.1 execution report are present.

## Approach

§ 4.1
Keep future markup and behavior as separate atomic units, define one semantic contract proof and one bounded direct-interaction browser story, record the user-authorized planning-time browser exclusion without fabricated evidence, conduct a separate sequential adversarial pass, resolve findings, validate structurally, run bounded context audit and benchmark checks, initialize plan context, create trackers, and write the execution report.

## Scope

§ 5.1
In scope: future button-chain.html #button-chain markup, appendButtonChain() behavior, generated-button counting, vertical placement, document clearing, exact finished text, white border, semantic and browser verification contracts, UI story/cache, review, validation, progress, and handoff.

§ 5.2
Out of scope during this proof: creating, editing, opening, serving, or testing any HTML; inspecting an HTML file; starting a browser, server, or driver; running the future UI flow; implementing code; using subagents or parallel workers; and modifying or deleting any pre-existing file.

## Affected areas

§ 6.1
Future implementation target only: button-chain.html at the repository root, with the named #button-chain DOM subtree and appendButtonChain() behavior scope. The root path is an explicit plan decision from the requested filename.

§ 6.2
Current durable artifacts are confined to basic-test-proof-1.4.1-isolated-plan-rerun: plan description, inventory, two goals and seven step documents, testing companions, UI story/cache/bug artifacts, adversarial review, context snapshot, progress trackers, and 1.4.1-analyze.md.

## Constraints and decisions

§ 7.1
Work is strictly sequential with no secondary agent because the user explicitly prohibits subagents and parallel workers. This conflicts with the skill preference for a fresh independent reviewer, so the plan uses a disclosed separate sequential adversarial pass and does not claim reviewer independence.

§ 7.2
The phrase fourth generated button means the fourth button appended after the initial button. The initial button press creates generated button 1; presses on generated buttons 1, 2, and 3 create generated buttons 2, 3, and 4; pressing generated button 4 performs completion and appends nothing.

§ 7.3
Below means a vertical document-order stack with each newly generated button on a new line beneath its predecessor. Completion removes every prior node from the document and leaves only a visible finished presentation whose containing rendered box has a white border.

## Risks and open questions

§ 8.1
No material open question blocks execution. A future executor must preserve exact lowercase finished text and distinguish generated-button count from total-button count. Browser evidence remains intentionally absent until the user lifts the planning-only boundary.

§ 8.2
A white border can be invisible on a white page; future browser proof must use a contrasting surrounding or completion background only if needed while retaining a genuinely white rendered border. Any added styling remains inside the W01 named subtree rather than becoming an unnamed target.

## UI classification

- UI affected: yes
- Rationale: The exact future task creates and changes an HTML interaction and visible completion state.

## UI validation

- Required: yes
- Browser target: Future executor opens repository-root button-chain.html in a real rendered browser; prohibited during this planning-only proof
- Story artifact: `ui-user-stories.md`

## Adversarial review

- Artifact: `adversarial-review.md`
- Status: ✅ approved
