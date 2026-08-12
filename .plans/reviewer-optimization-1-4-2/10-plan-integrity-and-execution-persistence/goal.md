# Goal: Enforce complete plan mutations and persistent execution

## Current state and prior-goal handoffs

§ 2.1
The plan has validator-backed work-unit ownership and bounded monitoring helpers, but a worker can still discover new scope and record it as an informal note, and a monitor can stop after a subprocess emits a status report even though the subprocess has not completed.

## Outcome and definition of done

§ 3.1
Every newly discovered scope item is converted into a complete, linked, validator-passing plan unit before it is treated as in scope. Monitors continue steering active subprocesses through intermediate status reports until explicit terminal evidence, a bounded retry limit, or a recorded blocker is reached.

## Why this goal is needed

§ 4.1
Note-only plan additions are not resumable or auditable, and status-only worker messages can leave implementation halted while appearing superficially complete. The protocol needs explicit integrity and persistence rules.

## Scope

§ 5.1
Include planning-skill mutation guidance and benchmark monitor/operator guidance. Exclude unbounded retries, blind process restarts, ignoring real failures, and changes to unrelated product behavior.

## Affected files, systems, data, and interfaces

§ 6.1
W64 updates the repository-local planning contract. W65 updates the benchmark operator/monitor contract. Evidence includes plan-validator output, adversarial-review status, process state, last worker output, next action, retry/steering count, and terminal reason.

## Dependencies and handoffs

§ 7.1
Depends on the existing helper-budget and runner-argument contracts (W48 and W61) plus cancellation ownership (W39). Hands off a validated plan mutation contract and a monitor continuation protocol to future implementation and pilot work.

## Implementation approach, risks, and edge cases

§ 8.1
Define terminal states explicitly: successful worker completion, accepted/tainted/rejected archive, clean process exit, or recorded blocker after bounded retries. A status message, partial report, or idle-looking interval is not terminal. On newly added scope, require the complete artifact chain and re-run validation before resuming implementation. Preserve evidence when a process is interrupted or genuinely blocked.

## Owned work units

§ 9.1
`W64` — Require complete validator-backed plan mutations for newly discovered scope.

§ 9.2
`W65` — Require persistent, evidence-driven monitor steering through intermediate subprocess states.

§ 9.3
`W66` — Test plan-mutation completeness and monitor continuation/terminal-state behavior with focused fixtures.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | Both contracts change executable planning/benchmark behavior and require focused validation for note-only additions, status-only output, continuation, bounded retries, and terminal evidence. |

## Goal-size exception

§ 11.1
Not applicable: this goal owns two closely related protocol-hardening outcomes with separate file ownership and verification.
