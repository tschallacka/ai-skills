# Goal: Define the button-chain implementation and proof contract

## Current state and prior-goal handoffs

§ 2.1
No implementation exists in the isolated workspace. The exact future behavior is supplied by the benchmark task: one initial button, one appended button per press of the current last button, terminal clearing on the fourth generated button, and bordered lowercase finished output.

## Outcome and definition of done

§ 3.1
A future executor can implement the single-file button chain and verify its terminal state from atomic, reviewable work units.

## Why this goal is needed

§ 4.1
This goal converts the future UI contract into independently reviewable targets and a bounded proof flow so another executor can implement it without inferring hidden files, symbols, counts, or visual acceptance criteria.

## Scope

§ 5.1
<included and explicitly excluded behavior>

## Affected files, systems, data, and interfaces

§ 6.1
<concrete affected areas>

## Dependencies and handoffs

§ 7.1
<prerequisites and precise downstream handoffs>

## Implementation approach, risks, and edge cases

§ 8.1
<approach, risks, and edge cases>

## Owned work units

§ 9.1
`W01` — Create the initial document structure containing exactly one initial button and the container needed for the interaction.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The plan owns a verification work unit and the future behavior is observable through direct browser input; this proof records the required flow but cannot execute it under the user-specified no-browser boundary. |

§ 9.2
`W02` — Append exactly one button after the current last button, preserving vertical DOM order.

§ 9.3
`W03` — Count generated-button activations and clear the document on the fourth generated button.

§ 9.4
`W04` — Give the finished text a visible white border while preserving the exact lowercase text.

§ 9.5
`W05` — Verify initial count, one-at-a-time append behavior, fourth-button clearing, exact finished text, and visible white border through direct UI input.

## Goal-size exception

§ 11.1
<required only when this goal has one permitted work unit>
