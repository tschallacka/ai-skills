#!/usr/bin/env bash
# MODE: DEV
# test-plan-context-arguments.sh — argument errors beat plan-state errors.

set -euo pipefail
export LC_ALL=C

tests_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$tests_dir/../.." && pwd)"
scripts="$repo_root/planning/scripts"
# shellcheck source=planning/tests/lib-test.sh
source "$tests_dir/lib-test.sh"
t_begin

work="$(mktemp -d "${TMPDIR:-/tmp}/plan-context-args.XXXXXX")"
trap 'rm -rf "$work"' EXIT
missing="$work/missing-plan"

expect_usage_before_missing() {
    local label="$1"
    shift
    local rc=0 out
    out="$(PLANNING_CONTEXT_CACHE=1 "$scripts/plan-context.sh" "$@" 2>&1)" || rc=$?
    t_assert_eq "$label exits 2" "$rc" 2
    case "$out" in
        *'usage:'*|*'Usage:'*) ;;
        *) t_fail "$label did not report usage: $out" ;;
    esac
    case "$out" in
        *'not-found: plan directory'*) t_fail "$label checked the plan directory before arguments" ;;
    esac
}

expect_usage_before_missing "bad format" read --plan-dir "$missing" --document plan --format xml
expect_usage_before_missing "bad limit" read --plan-dir "$missing" --document plan --max-records 0
expect_usage_before_missing "malformed token" read --plan-dir "$missing" --document plan --token bad
expect_usage_before_missing "missing read selector" read --plan-dir "$missing"
expect_usage_before_missing "multiple check selectors" check --plan-dir "$missing" --changed --all

t_end "test-plan-context-arguments"
