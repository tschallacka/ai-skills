<!-- MODE: DEV -->
# The compiled-binary-preference wiring needs plan-core-lib.sh, which does not exist on a fresh checkout

Measured 2026-09-16, discovered via a real CI run on a genuinely fresh
`nextupdate` checkout (run 35145081444), after seven scripts wired this
session all sourced `planning/scripts/plan-core-lib.sh` unconditionally.

## The fact

Every caller of `plan_exec_compiled_binary_if_present` reaches it by
`source`-ing `planning/scripts/plan-core-lib.sh` first. That file is
**generated** by `build-plan-libs.sh` and **gitignored** — it does not exist
on a clone that has never bootstrapped. A bare `source
".../plan-core-lib.sh"` with no existence check therefore fails outright
under `set -e`/`set -u`, before the caller ever reaches its own bash
implementation:

    blast-radius.sh: line 91: .../plan-core-lib.sh: No such file or directory

This is a **regression the wiring itself introduces**: before a script is
wired, `bash <script>.sh` has zero dependency on any generated file and
works on any checkout. After wiring, it silently gains a hard dependency on
`plan-core-lib.sh` existing — even though the wiring's own job (check for a
compiled binary, else fall through) has nothing to do with that file's
actual contents.

It went unnoticed through nine goals' worth of "full regression sweep"
claims (T145 goals 14-22) because every local dev environment in this
session already had a stale-but-present `plan-core-lib.sh` left over from
earlier work — this class of bug is invisible until tested against a truly
clean checkout, which local `run-tests.sh` sweeps never are.

## The fix (already applied where found)

Guard the `source` + `plan_exec_compiled_binary_if_present` call on the
file's own existence, falling through unconditionally to the bash body when
it is absent — the same pattern B346 established for `build-plan-libs.sh`'s
own self-referential case (it is one of the five files that script itself
generates):

```bash
if [ -f "$script_dir/planning/scripts/plan-core-lib.sh" ]; then
    source "$script_dir/planning/scripts/plan-core-lib.sh"
    plan_exec_compiled_binary_if_present <name> "$script_dir" "$@"
fi
```

Fixed for `verify-both-shells.sh`, `setup-dev-env.sh`,
`generate-portability.sh`, `blast-radius.sh`, `.github/ci-scope.sh`,
`.github/ci-subjects.sh`, `.github/ci-test-scope.sh` (commit
`b7f559e4`).

## What this means for other callers

At the same audit (2026-09-16), roughly 60 more already-shipped scripts —
every T146/T147 `planning/scripts/*.sh` entry point, `run-tests.sh`,
`pre-push-check.sh`, `ci-failures/scripts/ci-failures.sh` — source
`plan-core-lib.sh` the same unguarded way. Whether that is a live problem
for them (versus always running after a bootstrap step that already
generated the file) was **not established** in this session and needs its
own check before assuming it is fine. Any new script wired onto
`plan_exec_compiled_binary_if_present` should apply the guard above from
the start rather than risk rediscovering this the same way.
