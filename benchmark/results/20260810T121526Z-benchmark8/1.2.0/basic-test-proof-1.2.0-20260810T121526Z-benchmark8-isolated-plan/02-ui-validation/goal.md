# Goal 02: ui-validation

## Current state and prior-goal handoffs

Goal 01 defines the deterministic future HTML contract and supplies validator, artifact, and process audit handoffs. No HTML has been created or executed in this proof.

## Outcome and definition of done

Complete the final UI-validation handoff for US-01 and record story, cache, bug, and benchmark report evidence. Done only when the future story passes with direct browser evidence and no unresolved bug, or when a user-approved exclusion exists; this proof intentionally leaves it incomplete because browser execution is forbidden.

## Why this goal is needed

The planning skill requires a final UI-validation goal so the observable user journey remains an executable acceptance contract rather than prose.

## Scope

In scope are US-01 browser verification, its story row, run cache, bug register, and analysis report. Out of scope are HTML implementation changes and any browser execution during this proof.

## Affected files, systems, data, and interfaces

Future workspace-root `button-chain.html`, local browser route, `ui-user-stories.md`, `ui-story-runs/US-01.md`, `bugs.md`, and `analysis-report.md`.

## Dependencies and handoffs

W05 consumes W01–W04 and Goal 01's static handoff. W11–W13 preserve the separate result artifacts. W08 consumes all evidence and records the final benchmark handoff.

## Implementation approach, risks, and edge cases

Run exactly five clicks: `Button 0`, `Button 1`, `Button 2`, `Button 3`, then terminal generated `Button 4`. Record one-button increments after the first four clicks, clearing after the fifth, exact lowercase text, white border, route, and screenshot. If the story fails, retain the bug row and add investigation/fix/retest goals before completion.

## Owned work units

W05, W08, W11, W12, and W13 are owned by this final UI-validation goal. They separately own the browser flow, analysis report, story row, run cache, and bug register.
