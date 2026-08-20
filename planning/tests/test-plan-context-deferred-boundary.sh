#!/usr/bin/env bash
# MODE: DEV
# PACKAGE: DEV
set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
# Report the failing expression: these assertions are bare `[ ... ]` under
# set -e, which otherwise exits 1 in silence.
t_trap_assertions


repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
if [ -z "${PLANNING_CONTEXT_CACHE:-}" ]; then
    printf '%s\n' 'test-plan-context-deferred-boundary: ERROR (PLANNING_CONTEXT_CACHE is required)' >&2
    exit 64
fi
fixture="$PLANNING_CONTEXT_CACHE"
if [ ! -d "$fixture" ]; then
    printf 'test-plan-context-deferred-boundary: ERROR (fixture unavailable: %s)\n' "$fixture" >&2
    exit 66
fi
cp -R "$fixture" "$tmp/plan"
"$repo_dir/planning/scripts/plan-context.sh" init --plan-dir "$tmp/plan" >/dev/null
"$repo_dir/planning/scripts/plan-context.sh" read --plan-dir "$tmp/plan" --document plan --read-only >/dev/null
"$repo_dir/planning/scripts/plan-context.sh" check --plan-dir "$tmp/plan" --all >/dev/null
"$repo_dir/planning/scripts/plan-context.sh" refresh --plan-dir "$tmp/plan" --stale >/dev/null
"$repo_dir/planning/scripts/update-plan-content.sh" --description-paragraph "$tmp/plan" 2.1 'Mutated through the canonical helper.' >/dev/null
[ -s "$tmp/plan/context/mutation-handoff" ]
for forbidden in .git registry versions changelog quarantine events compaction workers; do
    if [ -n "$(find "$tmp/plan" -name "$forbidden" -o -name "$forbidden.*" || true)" ]; then
        printf 'deferred state created: %s\n' "$forbidden" >&2
        exit 1
    fi
done
printf '%s\n' 'test-plan-context-deferred-boundary: PASS'
