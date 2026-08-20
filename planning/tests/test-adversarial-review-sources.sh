#!/usr/bin/env bash
# MODE: DEV
# test-adversarial-review-sources.sh — the three-way CSV source selection in
# update-adversarial-review.sh, and which source it is allowed to consume.
#
# adversarial-review-incoming.md is the one plan file a reviewer may write, so
# it must only be removed when it was the source that got rendered. A --file or
# stdin run must leave it untouched for the run that will consume it.

set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/planning-review-sources-test.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT

note_fail() { printf 'adversarial-review-sources: %s\n' "$1" >&2; t_record "$1"; }

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

# ---- every finding stays a finding, and the header stays a header ----------
# plan_render_csv_table emits the |---| delimiter after row 1, which is right for
# --table-paragraph where row 1 IS the header. This caller passes findings, so
# without a prepended header the first finding of every cycle became the header:
# the table lost its column names and AR-nn was presented as one. Every test here
# passed while that was true, which is why this case exists.
header_probe="$temporary_root/plan-header"
mkdir -p "$header_probe"
"$script_dir/create-adversarial-review.sh" "$header_probe" >/dev/null
printf 'AR-41,First gap.,Do the first thing.,%s,W01\nAR-42,Second gap.,Do the second thing.,%s,W02\n' \
    '✅ resolved' '✅ resolved' \
    | "$script_dir/update-adversarial-review.sh" "$header_probe" >/dev/null 2>&1 \
    || note_fail 'the rewrite failed on two findings'
findings_table="$header_probe/adversarial-review.md"
grep -Fqx '| ID | Missing or over-broad item | Required plan change | Status | Work unit |' "$findings_table" \
    || note_fail 'the rewritten Findings table lost its column header'
for recorded in AR-41 AR-42; do
    grep -Fq "| $recorded |" "$findings_table" \
        || note_fail "$recorded is not in the rewritten Findings table"
done
# The delimiter must follow the header, not the first finding.
awk '/^\| ID \|/ { getline next_line; if (next_line !~ /^\|---\|/) exit 1 }' "$findings_table" \
    || note_fail 'the table delimiter does not follow the header row'
# And the gate that reads this table must see both pairs, not one.
# The helper lives in the library the scripts source, not in this test's shell.
pairs="$("$BASH" -c "source '$script_dir/plan-document-lib.sh'; plan_review_gated_pairs '$findings_table'" | grep -c . || true)"
[ "$pairs" -eq 2 ] \
    || note_fail "the fix-key gate sees $pairs gated pair(s) in a two-finding table"

# ---- the previous table is archived, and the notice says where -------------
# A rewrite replaces the whole Findings table. That content is a person's review
# work, so the run has to name where it went and the claim has to be true --
# a notice pointing at an archive that did not receive the rows is worse than
# silence. CODE-CONTRACTS.md contract 9a.
rc=0
archive_log="$temporary_root/archive-notice.log"
printf 'AR-31,A later gap.,Do the later thing.,%s,W01\n' '✅ resolved' \
    | "$script_dir/update-adversarial-review.sh" "$plan_stdin" \
    >"$archive_log" 2>&1 || rc=$?
[ "$rc" -eq 0 ] || note_fail "the second rewrite failed (rc=$rc)"
history_file="$plan_stdin/adversarial-review-history.md"
grep -Fq 'Archived previous Findings table' "$archive_log" \
    || note_fail 'the rewrite did not say the previous table was archived'
grep -Fq "$history_file" "$archive_log" \
    || note_fail 'the archive notice did not name the file it wrote to'
if [ -f "$history_file" ]; then
    grep -Fq '| AR-30 |' "$history_file" \
        || note_fail 'the archive does not hold the rows the notice claimed it archived'
else
    note_fail "the notice named an archive that does not exist: $history_file"
fi
grep -Fq '| AR-30 |' "$plan_stdin/adversarial-review.md" \
    && note_fail 'the replaced row is still in the live findings table'

[ "$(t_failures)" -eq 0 ] || exit 1
printf 'test-adversarial-review-sources.sh passed.\n'
