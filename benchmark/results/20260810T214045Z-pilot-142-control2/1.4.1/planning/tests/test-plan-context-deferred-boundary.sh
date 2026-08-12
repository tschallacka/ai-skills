#!/usr/bin/env bash
set -euo pipefail

repo_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
context_cache="${PLANNING_CONTEXT_CACHE:-${HOME}/.codex/skills/planning/plans/planning-context-cache}"
if [ ! -d "$context_cache" ]; then
    printf 'test-plan-context: SKIPPED (fixture unavailable: %s)\n' "$context_cache"
    exit 0
fi
cp -R "$context_cache" "$tmp/plan"
"$repo_dir/planning/scripts/plan-context.sh" init --plan-dir "$tmp/plan" >/dev/null
"$repo_dir/planning/scripts/plan-context.sh" read --plan-dir "$tmp/plan" --document plan --read-only >/dev/null
"$repo_dir/planning/scripts/plan-context.sh" check --plan-dir "$tmp/plan" --all >/dev/null
"$repo_dir/planning/scripts/plan-context.sh" refresh --plan-dir "$tmp/plan" --stale >/dev/null
"$repo_dir/planning/scripts/update-plan-content.sh" --description-paragraph "$tmp/plan" 2.1 'Mutated through the canonical helper.' >/dev/null
[ -s "$tmp/plan/context/mutation-handoff" ]
for forbidden in .git registry versions changelog quarantine events compaction workers; do
    if find "$tmp/plan" -name "$forbidden" -o -name "$forbidden.*" | grep -q .; then
        printf 'deferred state created: %s\n' "$forbidden" >&2
        exit 1
    fi
done
printf '%s\n' 'test-plan-context-deferred-boundary: PASS'
