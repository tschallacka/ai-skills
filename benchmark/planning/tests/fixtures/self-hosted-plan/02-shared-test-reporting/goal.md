# Goal: The reporting tests share one reporter implementation

## Current state and prior-goal handoffs

§ 2.1
<confirmed facts and prerequisite handoffs>

## Outcome and definition of done

§ 3.1
The 33 tests that already report findings use the shared t_fail/t_end implementation, so a finding raised inside a command substitution is not discarded.

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
