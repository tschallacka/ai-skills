# Goal: Reduce repeated command overhead safely

## Current state and prior-goal handoffs

§ 2.1
The benchmark and validation workflow contains long monitoring commands that may be executed repeatedly while a run is in progress. W59 and W61 define the pilot matrix and command arguments that this execution-hygiene step may observe.

## Outcome and definition of done

§ 3.1
Repeated long monitoring or validation commands use a bounded executable helper under `/tmp` with explicit arguments, concise output, recorded invocation evidence, and appropriate cleanup. A run with no repeated long command records `not needed` and leaves no helper behind.

## Why this goal is needed

§ 4.1
Writing a long command once and invoking it by path reduces repeated conversation/tool-call text while making polling behavior reproducible and auditable.

## Scope

§ 5.1
Include run-specific read-only monitoring, validation, and evidence-inspection helpers. Exclude repository scripts, published archives, secrets, broad process control, and unrelated command refactors.

## Affected files, systems, data, and interfaces

§ 6.1
The helper is temporary runtime state under `/tmp/<run-id>` or an equivalent narrow temporary scope; its path, arguments, output limit, and cleanup status belong in run evidence. No repository or published-result file is changed.

## Dependencies and handoffs

§ 7.1
Depends on W59 and W61 for the pilot run identity and command argument contract. Hands off helper evidence or a documented `not needed` result to the pilot/release record.

## Implementation approach, risks, and edge cases

§ 8.1
Use a shebang, strict shell settings where compatible, quoted arguments, bounded output, and read-only inspection by default. Remove run-specific helpers after use. Treat a helper that broadens scope, leaks sensitive data, or remains in the repository/archive as a failure.

## Owned work units

§ 9.1
`W62` — Convert any long-running or repeatedly executed monitoring/benchmark command into a bounded executable helper under `/tmp`, accepting explicit arguments, so repeated observation does not duplicate a large command in the working conversation.

## Testing requirement

| Test required | Rationale |
|---|---|
| yes | The goal governs executable temporary helpers and requires verification of safety, bounded output, evidence, and cleanup. |

## Goal-size exception

§ 11.1
Allowed single-unit goal: this goal has one cohesive execution-hygiene outcome and one verification work unit.
