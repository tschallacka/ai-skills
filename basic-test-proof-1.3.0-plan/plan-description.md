# Plan: Basic test proof 1.3.1: button-chain implementation plan

## Current state

§ 2.1
The source brief requires a planning-only proof, now continued with corrected planning helper revision 1.3.1. The repository contains the planning skill and bundled helpers; no .codegraph directory or browser connector is available. The existing 1.4.0 proof establishes the comparable button-chain planning scope. No HTML, browser, server, or implementation artifact exists for this proof.

## Desired outcome

§ 3.1
Produce a resumable plan for button-chain.html: one initial button; clicking the current last button appends exactly one button below it; activating the fourth generated button clears the document and displays finished with a visible white border. The plan must include implementation targets, browser acceptance, review/adversarial-review evidence, progress, handoff, timestamps, and token-cost evidence or an explicit unavailable result.

## Approach

§ 4.1
Use the helper-created inventory as the scope source of truth, keep markup and behavior as separate future work units, define one bounded direct-click UI story and cache, exclude browser/HTML execution during this proof by the explicit safety boundary, perform a separately conducted sequential adversarial review because subagents are prohibited, approve synchronized review status, validate structurally, and complete the planning handoff.

## Scope

§ 5.1
In scope: the future button-chain.html output contract, its #button-chain markup, appendButtonChain() behavior, fourth-generated-button completion state, direct browser-click acceptance flow, review artifacts, validator proof, progress tracking, and handoff.

§ 5.2
Explicitly out of scope for this proof: creating, editing, serving, opening, or testing button-chain.html; starting a browser or server; changing application implementation; spawning another worker; and changing pre-existing repository files.

## Affected areas

§ 6.1
Future implementation target: button-chain.html at the repository root, with the named #button-chain DOM subtree and appendButtonChain() behavior scope. This path is an execution-plan decision supported by the comparable 1.4.0 proof; the brief does not prescribe a filename.

§ 6.2
Planning artifacts under this directory: plan-description.md, work-unit-inventory.md, two goal documents with atomic steps and testing companions, ui-user-stories.md, ui-story-runs/US-01.md, bugs.md, adversarial-review.md, progress.md, goal progress files, and 1.3.1-analyze.md.

## Constraints and decisions

§ 7.1
The user requires planning-only execution, shell-command file operations, sequential work, no subagent, no HTML/browser/server/implementation artifact, process cleanup, exact reporting, completion of the adversarial-review cycle, and continuation under corrected helper revision 1.3.1. The user-approved browser exclusion is recorded in US-01 evidence and its cache.

§ 7.2
Testing requirement is yes for both goals because the future UI behavior is observable; the proof uses plan-level structural validation now and gives the future executor exact browser verification instructions. Token cost is reported as unavailable unless local history exposes it.

## Risks and open questions

§ 8.1
The future executor must confirm that button-chain.html is the intended repository-root path before implementation; if a project convention requires another path, reopen review and update the named work units and story cache. Browser evidence remains pending by design until the prohibited planning-time execution boundary is lifted.

§ 8.2
The brief says fourth generated button; the plan interprets this as the fourth newly appended button activation, after the initial button, and states the counting rule in W02 and US-01.

## UI classification

- UI affected: yes
- Rationale: <why>

## UI validation

- Required: yes
- Browser target: No browser connector available; future executor must use a real rendered browser route
- Story artifact: `ui-user-stories.md`

## Adversarial review

- Artifact: `adversarial-review.md`
- Status: ✅ approved
