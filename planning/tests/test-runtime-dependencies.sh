#!/usr/bin/env bash
# MODE: DEV
# test-runtime-dependencies — a shipped script that needs rjq must refuse without
# it, not quietly do less.
#
# Usage: test-runtime-dependencies.sh
#
# rjq is a declared requirement of the planning skill (install.sh
# runtime_requirements), and the installer refuses to install without it. But a
# hand-copied skill directory never went through the installer, and every rjq
# call in the validate-plan pass libraries is `2>/dev/null`. Measured on a real
# plan before the guard existed: 14 findings with rjq, 2 without, exit 127 with
# no explanation. A gate that quietly stops enforcing is worse than one that
# refuses to run, so the entry points check up front and exit 69.
set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
scripts="$root/scripts"
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/planning-runtime-deps.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT

note_fail() { printf 'runtime-deps: %s\n' "$1" >&2; t_record "$1"; }

# A PATH that is complete except for rjq. Mirroring the whole PATH would be slow,
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
if [ -e "$jqless_bin/rjq" ]; then
    note_fail 'the rjq-less PATH unexpectedly contains rjq'
fi

# Exercise a copied skill tree so the checkout's locally built fallback binary
# cannot satisfy the dependency probe.
runtime_scripts="$temporary_root/scripts"
t_copy_tree "$scripts" "$runtime_scripts"

run_without_jq() {
    env -i PATH="$jqless_bin" AI_SKILLS_BIN_ROOT="$jqless_bin" HOME="${HOME:-/tmp}" TMPDIR="$temporary_root" \
        "$jqless_bin/bash" "$@" 2>&1
}

# A minimal plan is enough: the guard must fire before any plan content is read.
plan_dir="$temporary_root/plan"
PLANS_ROOT="$temporary_root" "$scripts/create-plan.sh" "$plan_dir" 'Dependency guard' >/dev/null

# validate-plan.sh: refuses with 69 and names rjq.
set +e
output="$(run_without_jq "$runtime_scripts/validate-plan.sh" "$plan_dir")"
rc=$?
set -e
[ "$rc" -eq 69 ] || note_fail "validate-plan.sh without rjq exited $rc, expected 69"
case "$output" in
    *rjq*) ;;
    *) note_fail "validate-plan.sh without rjq did not mention rjq: $output" ;;
esac
# It must refuse rather than report findings, or a caller cannot tell a broken
# install from a bad plan.
case "$output" in
    *FAIL:*) note_fail 'validate-plan.sh without rjq reported findings instead of refusing' ;;
esac

# register-command.sh: same contract.
set +e
output="$(run_without_jq "$runtime_scripts/register-command.sh" "$plan_dir" build 'make all' 'when building')"
rc=$?
set -e
[ "$rc" -eq 69 ] || note_fail "register-command.sh without rjq exited $rc, expected 69"
case "$output" in
    *rjq*) ;;
    *) note_fail "register-command.sh without rjq did not mention rjq: $output" ;;
esac

# With rjq present the same commands must work, so the guard cannot be a
# permanent refusal caused by a broken probe.
if command -v rjq >/dev/null 2>&1; then
    set +e
    "$scripts/validate-plan.sh" "$plan_dir" >/dev/null 2>&1
    rc=$?
    set -e
    # A fresh plan legitimately fails validation (1); it must not be 69.
    [ "$rc" -ne 69 ] || note_fail 'validate-plan.sh reported a missing rjq while rjq is installed'
else
    printf 'runtime-deps: rjq is not installed here; skipped the positive case\n' >&2
fi

# Every shipped script that calls rjq must either guard it itself or be reachable
# only through an entry point that does. The pass libraries are sourced by
# validate-plan.sh, which guards; a NEW rjq caller outside that set needs its own.
guarded_by_entry_point='validate-plan-commands-lib.sh validate-plan-comparisons-lib.sh validate-plan-goals-lib.sh validate-plan-placeholders-lib.sh validate-plan-serve-lib.sh'
while IFS= read -r script; do
    [ -n "$script" ] || continue
    name="$(basename "$script")"
    case " $guarded_by_entry_point " in *" $name "*) continue ;; esac
    grep -q 'command -v rjq' "$script" && continue
    # Delegating to a shared guard is better than copying the probe, and the
    # contract above already allows it -- "or be reachable only through an entry
    # point that does". Accept a named guard helper, but verify the helper
    # really guards rather than trusting the name: a guard that stopped probing
    # would otherwise excuse every caller that delegates to it.
    delegated=''
    guard='reg_require_jq'
    if grep -q "$guard" "$script"; then
        guard_src="$(grep -rl "^$guard()" "$scripts" 2>/dev/null | head -1)"
        if [ -n "$guard_src" ] && awk -v fn="$guard" '$0 ~ "^"fn"\\(\\)" {inside=1} inside && /command -v rjq/ {found=1} inside && /^}/ {inside=0} END {exit !found}' \
            "$guard_src"; then
            delegated=yes
        fi
    fi
    [ -n "$delegated" ] \
        || note_fail "$name calls rjq but neither guards it nor is covered by a guarded entry point"
# Full-line comments are excluded: this looks for scripts that CALL rjq, and a
# comment naming it is not a call. Matching prose made the gate fail on a
# library whose only mention of rjq was the sentence explaining where the
# binary is found. A trailing comment on a real command still counts.
done < <(awk '/^[[:space:]]*#/ { next } /(^|[^A-Za-z0-9_])rjq / { print FILENAME; nextfile }' \
    "$scripts"/*.sh 2>/dev/null || true)

[ "$(t_failures)" -eq 0 ] || exit 1
printf '%s\n' 'test-runtime-dependencies: PASS'
