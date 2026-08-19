#!/usr/bin/env bash
set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
# Report the failing expression: these assertions are bare `[ ... ]` under
# set -e, which otherwise exits 1 in silence.
t_trap_assertions


scripts="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT
"$scripts/create-plan.sh" "$tmp/plan" context >/dev/null
"$scripts/add-goal.sh" "$tmp/plan" 01-context context 'Bounded context proof.' >/dev/null
"$scripts/plan-context.sh" init --plan-dir "$tmp/plan" >/dev/null
grep -Fq 'Current state' <("$scripts/plan-context.sh" read --plan-dir "$tmp/plan" --document plan --view summary)
grep -Fq 'Current state' <("$scripts/plan-context.sh" read --plan-dir "$tmp/plan" --document goal:01-context --view changed-documents)
# --max-records bounds each PAGE exactly, and the withheld remainder is
# reported. An upper bound alone ("no more than 10 lines") is satisfied by a
# reader that silently drops most of the document, which is the defect this
# assertion used to hide; assert the exact page size and the resume token.
inventory_records="$(wc -l < "$tmp/plan/work-unit-inventory.md" | tr -d ' ')"
[ "$inventory_records" -gt 5 ] || { echo "FAIL: inventory fixture too small to page" >&2; exit 1; }
page="$("$scripts/plan-context.sh" read --plan-dir "$tmp/plan" --document inventory --max-records 5)"
records="$(printf '%s\n' "$page" | sed '/^next_token=/d' | wc -l | tr -d ' ')"
[ "$records" -eq 5 ] && echo "PASS: --max-records bounds one page to exactly 5 records" \
    || { echo "FAIL: --max-records page held $records records, expected 5" >&2; exit 1; }
case "$(printf '%s\n' "$page" | sed -n 's/^next_token=//p')" in
    continue:*) echo "PASS: a truncated inventory page reports a resume token" ;;
    *) echo "FAIL: withheld inventory records reported no next_token" >&2; exit 1 ;;
esac
whole="$("$scripts/plan-context.sh" read --plan-dir "$tmp/plan" --document inventory --max-records 100000)"
[ "$(printf '%s\n' "$whole" | wc -l | tr -d ' ')" -eq "$inventory_records" ] \
    && echo "PASS: a large --max-records returns the whole inventory" \
    || { echo "FAIL: --max-records could not grow past the view slice" >&2; exit 1; }
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
