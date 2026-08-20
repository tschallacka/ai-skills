#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: DEV
# test-inner-shell-consistency.sh — a script's own subshells must be the
# interpreter running it, not whatever `bash` PATH happens to offer.
# See PORTABILITY.md, rule `bash-by-path-lookup`.
#
# plan-context.sh spawned its init worker with `bash -c`. Under the test harness
# PATH resolves that to bash 3.2; run the same script directly and it resolves to
# the system bash. So the two halves of one script ran different interpreters,
# and `init` on a plan with no work-unit inventory exited 2 or 0 depending on
# which -- publishing a snapshot in only one case.
#
# The probe puts a bash on PATH that refuses to run. Anything resolving `bash`
# through PATH gets it; anything using "$BASH" never sees it.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/inner-shell.XXXXXX")"
trap 'rm -rf "$work"' EXIT

stub_bin="$work/stub"
mkdir -p "$stub_bin"
cat > "$stub_bin/bash" <<'STUB'
#!/bin/sh
echo "PATH's bash was used instead of the running interpreter" >&2
exit 99
STUB
chmod +x "$stub_bin/bash"

plan="$work/plan"
mkdir -p "$plan"
printf '# Plan\n\n## Current state\n\n%s 1.1 a plan with no inventory yet.\n' '§' \
    > "$plan/plan-description.md"

# init spawns a worker for the locked section: the one that used to be `bash -c`.
rc=0
out="$(PATH="$stub_bin:$PATH" "$BASH" "$repo_root/planning/scripts/plan-context.sh" \
    init --plan-dir "$plan" 2>&1)" || rc=$?
t_assert_eq 'init does not resolve its worker shell through PATH' "$rc" 0
case "$out" in
    *"PATH's bash was used"*)
        t_fail 'the init worker ran the stub from PATH, so it is not the interpreter running the script' ;;
esac

# The snapshot still has to be real, so this cannot pass by doing nothing.
generation="$(cat "$plan/context/current" 2>/dev/null || true)"
[ -n "$generation" ] || t_fail 'init published no snapshot generation'
index="$plan/context/snapshots/$generation/index.tsv"
if [ -f "$index" ]; then
    t_assert_eq 'and the snapshot indexes the plan document' \
        "$({ grep -c '^plan	' "$index" || true; })" 1
else
    t_fail "init published no index: $index"
fi

t_end
