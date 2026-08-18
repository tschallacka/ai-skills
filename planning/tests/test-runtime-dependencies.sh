#!/usr/bin/env bash
# test-runtime-dependencies — a shipped script that needs jq must refuse without
# it, not quietly do less.
#
# Usage: test-runtime-dependencies.sh
#
# jq is a declared requirement of the planning skill (install.sh
# runtime_requirements), and the installer refuses to install without it. But a
# hand-copied skill directory never went through the installer, and every jq
# call in the validate-plan pass libraries is `2>/dev/null`. Measured on a real
# plan before the guard existed: 14 findings with jq, 2 without, exit 127 with
# no explanation. A gate that quietly stops enforcing is worse than one that
# refuses to run, so the entry points check up front and exit 69.
set -euo pipefail
export LC_ALL=C

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
scripts="$root/scripts"
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/planning-runtime-deps.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT

fail=0
note_fail() { printf 'runtime-deps: %s\n' "$1" >&2; fail=1; }

# A PATH that is complete except for jq. Mirroring the whole PATH would be slow,
# so mirror the tools the scripts actually reach before and around the guard.
# Anything genuinely absent on this host is skipped rather than failing the test.
jqless_bin="$temporary_root/bin"
mkdir -p "$jqless_bin"
for tool in bash sh dirname basename cat cp mv rm mkdir rmdir mktemp ln \
    sed awk grep egrep fgrep sort uniq comm paste tr cut head tail wc od \
    find date id stat chmod cmp diff git printf env ls test expr \
    sha256sum shasum openssl; do
    path="$(command -v "$tool" 2>/dev/null || true)"
    [ -n "$path" ] || continue
    ln -sf "$path" "$jqless_bin/$tool"
done
if [ -e "$jqless_bin/jq" ]; then
    note_fail 'the jq-less PATH unexpectedly contains jq'
fi

run_without_jq() {
    env -i PATH="$jqless_bin" HOME="${HOME:-/tmp}" TMPDIR="$temporary_root" \
        "$jqless_bin/bash" "$@" 2>&1
}

# A minimal plan is enough: the guard must fire before any plan content is read.
plan_dir="$temporary_root/plan"
PLANS_ROOT="$temporary_root" "$scripts/create-plan.sh" "$plan_dir" 'Dependency guard' >/dev/null

# validate-plan.sh: refuses with 69 and names jq.
set +e
output="$(run_without_jq "$scripts/validate-plan.sh" "$plan_dir")"
rc=$?
set -e
[ "$rc" -eq 69 ] || note_fail "validate-plan.sh without jq exited $rc, expected 69"
case "$output" in
    *jq*) ;;
    *) note_fail "validate-plan.sh without jq did not mention jq: $output" ;;
esac
# It must refuse rather than report findings, or a caller cannot tell a broken
# install from a bad plan.
case "$output" in
    *FAIL:*) note_fail 'validate-plan.sh without jq reported findings instead of refusing' ;;
esac

# register-command.sh: same contract.
set +e
output="$(run_without_jq "$scripts/register-command.sh" "$plan_dir" build 'make all' 'when building')"
rc=$?
set -e
[ "$rc" -eq 69 ] || note_fail "register-command.sh without jq exited $rc, expected 69"
case "$output" in
    *jq*) ;;
    *) note_fail "register-command.sh without jq did not mention jq: $output" ;;
esac

# With jq present the same commands must work, so the guard cannot be a
# permanent refusal caused by a broken probe.
if command -v jq >/dev/null 2>&1; then
    set +e
    "$scripts/validate-plan.sh" "$plan_dir" >/dev/null 2>&1
    rc=$?
    set -e
    # A fresh plan legitimately fails validation (1); it must not be 69.
    [ "$rc" -ne 69 ] || note_fail 'validate-plan.sh reported a missing jq while jq is installed'
else
    printf 'runtime-deps: jq is not installed here; skipped the positive case\n' >&2
fi

# Every shipped script that calls jq must either guard it itself or be reachable
# only through an entry point that does. The pass libraries are sourced by
# validate-plan.sh, which guards; a NEW jq caller outside that set needs its own.
guarded_by_entry_point='validate-plan-commands-lib.sh validate-plan-placeholders-lib.sh validate-plan-serve-lib.sh'
while IFS= read -r script; do
    [ -n "$script" ] || continue
    name="$(basename "$script")"
    case " $guarded_by_entry_point " in *" $name "*) continue ;; esac
    grep -q 'command -v jq' "$script" \
        || note_fail "$name calls jq but neither guards it nor is covered by a guarded entry point"
done < <(grep -lE '(^|[^A-Za-z0-9_])jq ' "$scripts"/*.sh 2>/dev/null || true)

[ "$fail" -eq 0 ] || exit 1
printf '%s\n' 'test-runtime-dependencies: PASS'
