# Plan: Basic test proof plan 1.4.0 isolated

## Current state

§ 2.1
This is a fresh planning-only proof in revision 1.4.0. The workspace supplies benchmark-test.md, task-spec.md, and worker-prompt.md. The tagged task specification and tagged planning skill are the only planning sources used. No prior plan, repository implementation, HTML artifact, browser session, server, driver, or test result is in scope.

## Desired outcome

§ 3.1
The future executor can create button-chain.html with one initial button, append exactly one button immediately below the current final visible button for each activation through generated buttons 1–4, and when generated button 4 is pressed remove the button-chain root so zero buttons remain and visibly print exactly lowercase finished as the only application content with a non-zero solid white border on a contrasting background. This proof completion means the durable plan and evidence requirements are complete; it does not implement or validate HTML.

## Approach

§ 4.1
First define the atomic markup, completion style, and click-handler targets in Goal 01. Then execute the cached US-01 browser flow as Goal 02's separate verification unit. Review the complete draft adversarially, resolve findings, validate structurally, initialize bounded context and progress trackers, and record the planning-only result.

## Scope

§ 5.1
In scope: the future single-file HTML document, its initial button subtree, append and terminal interaction contract, visible finished styling, direct browser acceptance story, cache, testing companions, review, bug register, trackers, context snapshot, validator evidence, and analysis report. Explicitly out of scope for this proof: creating, editing, opening, serving, inspecting, or testing any HTML; browser/server/driver startup; repository discovery outside the two tagged skill/task paths; and implementation changes.

## Affected areas

§ 6.1
Future target: button-chain.html, divided into #button-chain-root markup, the button-chain initializer, the button-chain click handler, and .completion-message style. Plan artifacts include US-01, ui-story-runs/US-01.md, the five Goal 01 companions plus the W04 companion, and W04 browser verification.

## Constraints and decisions

§ 7.1
The benchmark requires the exact plan directory name basic-test-proof-1.4.0-20260810T121526Z-benchmark8-isolated-plan under this workspace. The tagged helper scripts own document structure. The current proof records untested browser status rather than inventing evidence. The exact future text is lowercase finished and the border must be visibly white.

## Risks and open questions

§ 8.1
Open questions are limited to execution-environment details for the future HTML run, such as the browser’s local-file navigation policy; these do not change the requested contract. Primary risks are off-by-one generated-button counting, stale current-last targeting, appending more than one control, failing to remove the root, and losing the white border when clearing the document; W03 and US-01 explicitly guard these cases. Future completion requires normal validation and, after the story passes with no open bugs, validate-plan.sh --complete.

## UI classification

- UI affected: yes
- Rationale: The future work creates an HTML UI, click interaction, and visible completion state; the tagged UI-story workflow is therefore mandatory.

## UI validation

- Required: yes
- Browser target: Local browser target: file://<workspace>/button-chain.html in a fresh browser context
- Story artifact: `ui-user-stories.md`

## Adversarial review

- Artifact: `adversarial-review.md`
- Status: ✅ approved
