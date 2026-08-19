#!/usr/bin/env bash
# test-adversarial-review-sources.sh — the three-way CSV source selection in
# update-adversarial-review.sh, and which source it is allowed to consume.
#
# adversarial-review-incoming.md is the one plan file a reviewer may write, so
# it must only be removed when it was the source that got rendered. A --file or
# stdin run must leave it untouched for the run that will consume it.

set -euo pipefail
export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/planning-review-sources-test.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT

fail=0
note_fail() { printf 'adversarial-review-sources: %s\n' "$1" >&2; fail=1; }

incoming_csv='ID,Missing or over-broad item,Required plan change,Status,Work unit
AR-20,from incoming,change it,✅ resolved,N/A
'
other_csv='ID,Missing or over-broad item,Required plan change,Status,Work unit
AR-30,from --file,change it,✅ resolved,N/A
'

seed_plan() {
    local plan="$1"
    mkdir -p "$plan"
    "$script_dir/create-adversarial-review.sh" "$plan" >/dev/null
    printf '%s' "$incoming_csv" > "$plan/adversarial-review-incoming.md"
}

# --- --file must not consume the reviewer's incoming file ---
plan_file="$temporary_root/plan-file"
seed_plan "$plan_file"
printf '%s' "$other_csv" > "$temporary_root/other.csv"
cp "$plan_file/adversarial-review-incoming.md" "$temporary_root/incoming-before.md"
rc=0
"$script_dir/update-adversarial-review.sh" "$plan_file" --file "$temporary_root/other.csv" \
    >/dev/null 2>&1 || rc=$?
[ "$rc" -eq 0 ] || note_fail "--file run failed (rc=$rc)"
if [ -f "$plan_file/adversarial-review-incoming.md" ]; then
    cmp -s "$temporary_root/incoming-before.md" "$plan_file/adversarial-review-incoming.md" \
        || note_fail '--file run modified the unconsumed incoming file'
else
    note_fail '--file run deleted the incoming file it never read'
fi
grep -Fq '| AR-30 |' "$plan_file/adversarial-review.md" \
    || note_fail '--file rows did not reach the findings table'
grep -Fq '| AR-20 |' "$plan_file/adversarial-review.md" \
    && note_fail '--file run rendered the incoming file instead of the named CSV'

# --- the surviving incoming file is consumed by the next run that reads it ---
# stdin is closed so a regressed selection reports empty input instead of
# blocking on a `cat` that nothing will ever feed.
rc=0
"$script_dir/update-adversarial-review.sh" "$plan_file" >/dev/null 2>&1 </dev/null || rc=$?
[ "$rc" -eq 0 ] || note_fail "incoming-source run failed (rc=$rc)"
grep -Fq '| AR-20 |' "$plan_file/adversarial-review.md" \
    || note_fail 'the preserved incoming file was not rendered by the next run'
[ -e "$plan_file/adversarial-review-incoming.md" ] \
    && note_fail 'a run that consumed the incoming file left it in place'

# --- stdin remains the last resort when no incoming file is present ---
plan_stdin="$temporary_root/plan-stdin"
mkdir -p "$plan_stdin"
"$script_dir/create-adversarial-review.sh" "$plan_stdin" >/dev/null
rc=0
printf '%s' "$other_csv" | "$script_dir/update-adversarial-review.sh" "$plan_stdin" \
    >/dev/null 2>&1 || rc=$?
[ "$rc" -eq 0 ] || note_fail "stdin run failed (rc=$rc)"
grep -Fq '| AR-30 |' "$plan_stdin/adversarial-review.md" \
    || note_fail 'stdin rows did not reach the findings table'

[ "$fail" -eq 0 ] || exit 1
printf 'test-adversarial-review-sources.sh passed.\n'
