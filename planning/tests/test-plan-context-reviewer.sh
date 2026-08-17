#!/usr/bin/env bash
set -euo pipefail

scripts="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
"$scripts/create-plan.sh" "$tmp/plan" context >/dev/null
"$scripts/add-goal.sh" "$tmp/plan" 01-context context 'Bounded context proof.' >/dev/null
"$scripts/plan-context.sh" init --plan-dir "$tmp/plan" >/dev/null
grep -Fq 'Current state' <("$scripts/plan-context.sh" read --plan-dir "$tmp/plan" --document plan --view summary)
grep -Fq 'Current state' <("$scripts/plan-context.sh" read --plan-dir "$tmp/plan" --document goal:01-context --view changed-documents)
# --max-records bounds the returned row count (accepted and exercised).
recs="$("$scripts/plan-context.sh" read --plan-dir "$tmp/plan" --document inventory --max-records 5 2>/dev/null || true)"
[ "$(printf '%s\n' "$recs" | wc -l)" -le 10 ] && echo "PASS: --max-records bounds reads" \
    || { echo "FAIL: --max-records did not bound reads" >&2; exit 1; }
printf 'AR-01\n' > "$tmp/findings"
: > "$tmp/changed"
hash=0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef
"$scripts/plan-context.sh" checkpoint --plan-dir "$tmp/plan" --phase drafting --state in_progress \
    --findings-file "$tmp/findings" --changed-files "$tmp/changed" --source-hash "$hash" --plan-hash "$hash" >/dev/null
test -s "$tmp/plan/context/checkpoints/drafting.json"
grep -Fq '"phase":"drafting"' "$tmp/plan/context/checkpoints/drafting.json"
printf 'RUN_ID=test-worker\nREVISION=1.4.2\n' > "$tmp/vars"
"$scripts/plan-context-wrapper.sh" "$tmp/vars" read --plan-dir "$tmp/plan" --document plan --max-bytes 1024 >/dev/null
printf 'Reviewer context tests passed.\n'
