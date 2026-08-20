#!/usr/bin/env bash
# MODE: DEV
# test-die-temp-file-cleanup.sh — a refused write leaves no temp file in the plan.
#
# The writer functions guard their temp file with `trap ... RETURN`, and a RETURN
# trap does not fire on exit. plan_die exits, so every validation failure after
# the temp file was created left a "<file>.tmp.<pid>" next to the target, inside
# the plan tree, where it was committed as review debris — a reviewer found one
# and reported it. plan_die now removes the temp files registered by those
# writers.
#
# An EXIT trap inside the library is not available: the top-level scripts set
# their own, and adding one would clobber theirs.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
scripts_dir="$repo_root/planning/scripts"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/die-temp-cleanup.XXXXXX")"
trap 'rm -rf "$work"' EXIT
plan="$work/plan"
cp -R "$repo_root/benchmark/planning/tests/fixtures/self-hosted-plan" "$plan"

leaked_count() { find "$plan" -name '*.tmp.*' | wc -l | tr -d ' '; }

t_assert_eq 'the fixture starts with no temp files' "$(leaked_count)" 0

# ---- a refusal raised after the temp file exists ----------------------------
# A section absent from the document: the writer creates its temp file, the awk
# pass finds no heading, and plan_die reports it.
rc=0
"$scripts_dir/update-plan-content.sh" -ss "$plan" \
    '01-plan-dir-synonym/01-step-confirm-hoister-testing' browser-verification \
    -p '3.1: a browser check the companion has no section for.' >/dev/null 2>&1 || rc=$?
[ "$rc" -ne 0 ] || t_fail 'the write was accepted, so no refusal path was exercised'
t_assert_eq 'a refused section write leaves no temp file' "$(leaked_count)" 0

# ---- a refusal raised before the temp file exists --------------------------
# Still has to exit cleanly: the cleanup registry is empty here, and an empty
# array expansion is unbound under set -u on bash 3.2.
rc=0
"$scripts_dir/update-plan-content.sh" -ds "$plan" no-such-section \
    -p '2.1: text.' >/dev/null 2>&1 || rc=$?
[ "$rc" -ne 0 ] || t_fail 'an unknown section id was accepted'
t_assert_eq 'an early refusal leaves no temp file either' "$(leaked_count)" 0

# ---- the success path is unaffected ----------------------------------------
rc=0
"$scripts_dir/update-plan-content.sh" -ds "$plan" current-state \
    -p '2.1: a rewritten current state.' >/dev/null 2>&1 || rc=$?
t_assert_eq 'a valid section write still succeeds' "$rc" 0
grep -Fq 'a rewritten current state.' "$plan/plan-description.md" \
    || t_fail 'the successful write did not land'
t_assert_eq 'and leaves no temp file' "$(leaked_count)" 0

t_end
