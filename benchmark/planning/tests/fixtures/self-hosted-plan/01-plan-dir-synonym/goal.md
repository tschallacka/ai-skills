# Goal: Every helper taking a plan directory accepts --plan-dir

## Current state and prior-goal handoffs

§ 2.1
<confirmed facts and prerequisite handoffs>

## Outcome and definition of done

§ 3.1
All 20 remaining helpers accept --plan-dir as a synonym for the positional plan directory, proven byte-identical to the positional form.

## Why this goal is needed

§ 4.1
<how this goal contributes to the initiative>

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
`W01` — No change; the hoister already exists and is the seam the other units use.

§ 9.2
`W02` — Source plan-document-lib.sh above first use of $1, then hoist --plan-dir to position 1.

§ 9.3
`W03` — Hoist at position 1 after command="$1"; shift, so every subcommand sees the plan directory positionally.

§ 9.4
`W04` — One case per converted helper: positional and --plan-dir produce identical trees, output and exit status.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | A differential case per helper is the only way to show the two argument forms agree; the flag is silently ignorable otherwise. |

## Goal-size exception
