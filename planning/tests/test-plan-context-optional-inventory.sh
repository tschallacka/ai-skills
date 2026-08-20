#!/usr/bin/env bash
# test-plan-context-optional-inventory.sh — `init` on a plan that has no
# work-unit inventory yet must succeed, and must do so identically on every
# shell. See CODE-CONTRACTS.md contract 12.
#
# plan_inventory_rows was called on work-unit-inventory.md with no -f guard,
# unlike every other optional document in the same index builder. Bare awk on a
# missing file writes its own message and exits 2, and whether that aborted the
# caller depended on the bash running the inner `bash -c`: init exited 2 under
# bash 5.3 and 0 under bash 3.2, publishing a snapshot either way.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
scripts_dir="$repo_root/planning/scripts"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/context-optional-inventory.XXXXXX")"
trap 'rm -rf "$work"' EXIT
plan="$work/plan"
mkdir -p "$plan"
printf '# Plan\n\n## Current state\n\n%s 1.1 an early plan, before its inventory exists.\n' \
    '§' > "$plan/plan-description.md"

# ---- init succeeds without an inventory ------------------------------------
rc=0
"$scripts_dir/plan-context.sh" init --plan-dir "$plan" >/dev/null 2>&1 || rc=$?
t_assert_eq 'init succeeds on a plan with no work-unit inventory' "$rc" 0

generation="$(cat "$plan/context/current" 2>/dev/null || true)"
[ -n "$generation" ] || t_fail 'init published no snapshot generation'
index="$plan/context/snapshots/$generation/index.tsv"
if [ -f "$index" ]; then
    t_assert_eq 'the snapshot indexes the plan document' \
        "$({ grep -c '^plan	' "$index" || true; })" 1
    t_assert_eq 'and indexes no work units' \
        "$({ grep -c '^unit:' "$index" || true; })" 0
else
    t_fail "init published no index: $index"
fi

# The snapshot has to be usable, not merely present.
rc=0
"$scripts_dir/plan-context.sh" read --plan-dir "$plan" --document plan >/dev/null 2>&1 || rc=$?
t_assert_eq 'the published snapshot can be read back' "$rc" 0

# ---- a caller that requires the inventory gets a diagnosis ------------------
# 66 is the repo's code for missing input, and the message has to name the
# remedy rather than leave awk to report "cannot open".
probe="$work/probe.sh"
cat > "$probe" <<'PROBE'
set -euo pipefail
. "$1/plan-inventory-lib.sh"
plan_inventory_rows "$2/work-unit-inventory.md"
PROBE
rc=0
message="$(bash "$probe" "$scripts_dir" "$plan" 2>&1)" || rc=$?
t_assert_eq 'a missing inventory is refused with 66' "$rc" 66
case "$message" in
    *'work-unit inventory not found'*) ;;
    *) t_fail "the refusal did not name the missing inventory: $message" ;;
esac
case "$message" in
    *create-work-unit-inventory.sh*) ;;
    *) t_fail "the refusal did not name the remedy: $message" ;;
esac

t_end
