# Goal: The reporting tests share one reporter implementation

## Current state and prior-goal handoffs

§ 2.1
Thirty-three test files each defined their own reporter over a local counter; twenty-six of them are counter-style and lose a finding raised inside a command substitution.

## Outcome and definition of done

§ 3.1
The 33 tests that already report findings use the shared t_fail/t_end implementation, so a finding raised inside a command substitution is not discarded.

## Why this goal is needed

§ 4.1
A suite that can report PASS over a real finding is worse than no suite, because it is trusted.

## Scope

§ 5.1
Included: the twenty-six counter-style reporters. Excluded: the seven that report and exit on the first failure, where continuing would run against broken state.

## Affected files, systems, data, and interfaces

§ 6.1
planning/tests/lib-test.sh as the shared seam, the twenty-six converted tests, and CODE-STYLE section 12 for the convention.

## Dependencies and handoffs

§ 7.1
Depends on t_record and t_failures in lib-test.sh. Hands off a suite whose findings survive a subshell.

## Implementation approach, risks, and edge cases

§ 8.1
Each converted test keeps its own message and prefix, so no output changes on the passing path. Risk: a sourced library that changes shell options alters test semantics, which is how errexit leaked into a test that deliberately runs without it.

## Owned work units

§ 9.1
`W05` — No change; the shared reporter already exists and is the seam the conversion targets.

§ 9.2
`W06` — Point the local reporter at t_fail and replace the counter epilogue with t_end, leaving every message and call site unchanged.

§ 9.3
`W07` — Every test stdout, stderr and exit status is byte-identical to the captured baseline on the passing path.

§ 9.4
`W08` — A planted failure in a converted test still names its finding and still exits non-zero.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The conversion touches the suite itself, so a captured baseline plus a mutation spot-check is the only evidence that behaviour held. |

## Goal-size exception
