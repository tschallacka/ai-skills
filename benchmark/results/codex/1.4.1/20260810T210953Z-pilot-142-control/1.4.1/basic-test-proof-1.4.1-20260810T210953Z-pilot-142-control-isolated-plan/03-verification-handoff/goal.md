# Goal: Verify and hand off browser behavior

## Current state and prior-goal handoffs

§ 2.1
This goal depends on `01-document-shell` and `02-chain-behavior`. The future executor may begin it only after the planned markup, styling, and click handler are implemented.

## Outcome and definition of done

§ 3.1
Static inspection and browser verification prove the button-chain acceptance criteria, and handoff evidence is ready for review.

## Why this goal is needed

§ 4.1
This goal proves the acceptance contract through bounded static inspection and direct browser story verification. It prevents the plan from relying on implementation intent without observable evidence.

## Scope

§ 5.1
Included: one bounded static acceptance inspection and one direct-click browser story run. Excluded: fixing discovered bugs inside verification steps; failures must open investigation and fix goals through the bug register.

## Affected files, systems, data, and interfaces

§ 6.1
Affected verification targets: the static source inspection command and UI story `US-01`. No implementation file is changed by this goal.

## Dependencies and handoffs

§ 7.1
Prerequisites: `W07` and `W08`. Handoff: verification evidence states whether the implementation satisfies the exact file, click, below-placement, terminal clear, lowercase text, and white-border requirements.

## Implementation approach, risks, and edge cases

§ 8.1
Run the cheapest source inspection first, then execute the cached browser story. If either check fails, update `bugs.md` and create follow-up investigation and fix goals before claiming completion.

## Owned work units

§ 9.1
`W04` — Inspect button-chain.html source after implementation for exact file, initial button, generated-button counter, lowercase finished text, below-button layout, and white-border CSS.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | This goal exists to run and record verification evidence. |

§ 9.2
`W05` — Run the cached direct-click browser story against button-chain.html and record pass/fail evidence.

## Goal-size exception

§ 11.1
Not applicable. This goal owns two verification work units.
