# Goal: Every helper taking a plan directory accepts --plan-dir

## Current state and prior-goal handoffs

§ 2.1
plan_hoist_plan_dir already existed and was proven on four argument shapes; twenty helpers still took the plan directory positionally.

## Outcome and definition of done

§ 3.1
All 20 remaining helpers accept --plan-dir as a synonym for the positional plan directory, proven byte-identical to the positional form.

## Why this goal is needed

§ 4.1
A reader learns --plan-dir from the bounded reader, which is their primary tool, then has the call refused by every other helper. This goal removes that asymmetry.

## Scope

§ 5.1
Included: the twenty positional helpers and update-plan-content subcommands. Excluded: cleanup-plans.sh, which takes a plans root, and the two helpers that already consume the flag natively.

## Affected files, systems, data, and interfaces

§ 6.1
planning/scripts/*.sh argument parsing, their usage text, and planning/tests/test-flag-form-equivalence.sh plus the new test-plan-dir-synonym.sh.

## Dependencies and handoffs

§ 7.1
Depends on plan_hoist_plan_dir in plan-document-lib.sh. Hands off a proven flag surface that the self-hosted plan fixture then exercises.

## Implementation approach, risks, and edge cases

§ 8.1
The hoist must sit after script_dir is defined or the function is undefined at call time. A differential test that only compares output will pass for two equally broken paths, so each case also asserts the invocation had an effect.

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
