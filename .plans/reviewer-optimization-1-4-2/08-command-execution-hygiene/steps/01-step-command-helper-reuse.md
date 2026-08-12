# Step: 01-step-command-helper-reuse

## Ownership

- Goal: `08-command-execution-hygiene`
- Work unit: `W62`
- Type: `verification`

## Change target

- File: `N/A`
- Primary symbol or file scope: `reusable monitoring command helper`
- Subscope: `N/A`

## Objective

Reduce repeated command-text and token overhead during long benchmark work while preserving reproducibility and safety.

## Instructions

1. When a command is long enough to be executed repeatedly, write it to a uniquely named executable helper under `/tmp`, using a narrow run-specific directory or filename. For monitor commands, expose a bounded selector flag such as `1`, `2`, or `3` to choose the process profile, followed by explicit run/case/result arguments.
2. Use a shebang, `set -euo pipefail` where compatible, explicit positional or named arguments, and quoted variable expansions. Do not embed unresolved user input or rely on broad globs for destructive operations.
3. Keep monitoring helpers read-only unless the requested workflow explicitly requires a bounded state change. Make output concise by default and provide arguments for the run ID, result path, polling interval, or output limit where useful.
4. Invoke the helper for subsequent polls/checks and record its path and arguments in the relevant working context or run evidence. Do not paste the full long command again merely to repeat it.
5. Remove the helper after the run when it contains run-specific data or secrets; otherwise retain only a clearly named, non-sensitive helper whose lifecycle is documented. Never place these temporary helpers in the repository or published benchmark archive.

## Acceptance criteria

- Repeated long monitoring/validation commands use an executable `/tmp` helper rather than duplicating the full command text.
- Helpers accept explicit arguments safely, produce bounded output, and do not broaden filesystem or process scope.
- Monitor helpers accept the documented selector flags and reject unsupported values before inspecting processes.
- The helper path, invocation contract, and cleanup result are recorded in run evidence.
- No helper is committed to the repository or included in a published plan/result archive.

## Handoff

Hand off the helper path or an explicit `not needed` determination, its invocation contract, bounded-output evidence, and cleanup status to the pilot/release evidence.

## Atomicity check

- [x] This step owns exactly one inventory work unit.
- [x] No other file, symbol, test target, or verification flow changes here.
- [x] Any follow-on target has a separately named work unit and step.
