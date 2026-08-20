#!/usr/bin/env bash
# MODE: PROD
# test-adversarial-review-cycles.sh — cycle numbering and the archive guard in
# update-adversarial-review.sh.
#
# The number is the highest recorded plus one, never a count: one explicit
# --cycle above the count would otherwise send every later number backwards over
# labels already in use. The archive dedupes on the row set, so an identical
# re-run does not file twice, and a table nothing has archived yet is refused
# rather than discarded when its number is taken.

set -euo pipefail
# shellcheck source=planning/tests/lib-test.sh
source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/lib-test.sh"
t_begin

export LC_ALL=C

scripts="$(cd "$(dirname "${BASH_SOURCE[0]}")/../scripts" && pwd)"
temporary_root="$(mktemp -d "${TMPDIR:-/tmp}/planning-review-cycles-test.XXXXXX")"
trap 'rm -rf "$temporary_root"' EXIT

note_fail() { printf 'adversarial-review-cycles: %s\n' "$1" >&2; t_record "$1"; }

# The heading numbers in file order, space-separated. An absent history and one
# with no headings are both legitimate answers, so grep is allowed to exit
# non-zero inside the pipe rather than on the assignment.
cycle_headings() {
    { grep -E '^## Cycle [0-9]+$' "$1" 2>/dev/null || true; } | awk '{ printf "%s ", $3 }'
}

assert_headings() {
    local actual
    actual="$(cycle_headings "$1")"
    [ "$actual" = "$2 " ] || note_fail "$3: headings were [$actual], expected [$2 ]"
}

assert_increasing() {
    local verdict
    verdict="$(cycle_headings "$1" | awk '
        {
            for (i = 1; i <= NF; i++) {
                if ($i + 0 <= previous + 0) { print "no"; exit }
                previous = $i
            }
            print "yes"
        }
    ')"
    [ "$verdict" = yes ] || note_fail "$2: headings are not strictly increasing: [$(cycle_headings "$1")]"
}

assert_count() {
    local actual
    actual="$({ grep -c -- "$2" "$3" 2>/dev/null || true; })"
    [ "${actual:-0}" -eq "$1" ] || note_fail "$4: found ${actual:-0} occurrences of $2, expected $1"
}

seed_plan() {
    mkdir -p "$1"
    "$scripts/create-adversarial-review.sh" "$1" >/dev/null
}

# One row, so the row set identifies the cycle it belongs to.
run_update() {
    local plan="$1" id="$2"
    shift 2
    printf 'ID,Missing or over-broad item,Required plan change,Status,Work unit\nAR-%s,the reviewer%ss gap %s,change the plan,open,N/A\n' \
        "$id" "'" "$id" \
        | "$scripts/update-adversarial-review.sh" "$plan" "$@" >/dev/null 2>&1
}

# --- a plain sequence of automatic runs numbers 1, 2, 3 and archives each ---
plan_auto="$temporary_root/plan-auto"
seed_plan "$plan_auto"
rc=0
run_update "$plan_auto" 91 || rc=$?
run_update "$plan_auto" 92 || rc=$?
run_update "$plan_auto" 93 || rc=$?
[ "$rc" -eq 0 ] || note_fail "an automatic run failed (rc=$rc)"
history_auto="$plan_auto/adversarial-review-history.md"
assert_headings "$history_auto" '1 2 3' 'three automatic runs'
assert_count 1 '| AR-91 |' "$history_auto" 'automatic runs'
assert_count 1 '| AR-92 |' "$history_auto" 'automatic runs'
assert_count 0 '| AR-93 |' "$history_auto" 'automatic runs archived the live table'

# --- an explicit --cycle must not renumber the automatic runs after it ---
plan_mixed="$temporary_root/plan-mixed"
seed_plan "$plan_mixed"
rc=0
run_update "$plan_mixed" 11 || rc=$?
run_update "$plan_mixed" 12 || rc=$?
run_update "$plan_mixed" 13 --cycle 10 || rc=$?
run_update "$plan_mixed" 14 || rc=$?
run_update "$plan_mixed" 15 || rc=$?
[ "$rc" -eq 0 ] || note_fail "a run in the mixed sequence failed (rc=$rc)"
history_mixed="$plan_mixed/adversarial-review-history.md"
assert_headings "$history_mixed" '1 2 10 11 12' 'auto after --cycle 10'
assert_increasing "$history_mixed" 'mixed sequence'
assert_count 0 '## Cycle 4' "$history_mixed" 'the number after --cycle 10 fell back to a count'

# --- an identical re-run must not archive the same rows twice ---
plan_same="$temporary_root/plan-same"
seed_plan "$plan_same"
rc=0
run_update "$plan_same" 20 || rc=$?
run_update "$plan_same" 20 || rc=$?
run_update "$plan_same" 20 || rc=$?
[ "$rc" -eq 0 ] || note_fail "an identical re-run failed (rc=$rc)"
history_same="$plan_same/adversarial-review-history.md"
assert_headings "$history_same" '1 2' 'identical re-runs'
assert_count 1 '| AR-20 |' "$history_same" 'identical re-runs'

# --- a differing table whose number collides is refused, never discarded ---
plan_clash="$temporary_root/plan-clash"
seed_plan "$plan_clash"
rc=0
run_update "$plan_clash" 30 --cycle 5 || rc=$?
[ "$rc" -eq 0 ] || note_fail "the first --cycle 5 run failed (rc=$rc)"
cp "$plan_clash/adversarial-review.md" "$temporary_root/review-before.md"
cp "$plan_clash/adversarial-review-history.md" "$temporary_root/history-before.md"
rc=0
printf 'ID,Missing or over-broad item,Required plan change,Status,Work unit\nAR-31,a different gap,change the plan,open,N/A\n' \
    | "$scripts/update-adversarial-review.sh" "$plan_clash" --cycle 5 \
    >"$temporary_root/clash.out" 2>"$temporary_root/clash.err" || rc=$?
[ "$rc" -eq 73 ] || note_fail "a colliding --cycle with different rows exited $rc, expected 73"
clash_message="$(cat "$temporary_root/clash.err")"
case "$clash_message" in
    *'Cycle 5'*) : ;;
    *) note_fail "the refusal does not name the colliding cycle: $clash_message" ;;
esac
cmp -s "$temporary_root/review-before.md" "$plan_clash/adversarial-review.md" \
    || note_fail 'a refused run overwrote the previous findings table'
cmp -s "$temporary_root/history-before.md" "$plan_clash/adversarial-review-history.md" \
    || note_fail 'a refused run still wrote to the history file'
assert_count 1 '| AR-30 |' "$plan_clash/adversarial-review.md" 'the refused run'

# --- the no-rows path still records a marker ---
plan_empty="$temporary_root/plan-empty"
seed_plan "$plan_empty"
awk '
    /^## Findings$/ { in_findings = 1; print; next }
    in_findings && /^## Verdict$/ { in_findings = 0; print; next }
    in_findings && /^\|/ { next }
    { print }
' "$plan_empty/adversarial-review.md" > "$temporary_root/empty-findings.md"
mv "$temporary_root/empty-findings.md" "$plan_empty/adversarial-review.md"
rc=0
run_update "$plan_empty" 40 || rc=$?
[ "$rc" -eq 0 ] || note_fail "the no-rows run failed (rc=$rc)"
history_empty="$plan_empty/adversarial-review-history.md"
assert_headings "$history_empty" '1' 'the no-rows run'
assert_count 1 '_No row-level findings were recorded for this cycle._' "$history_empty" 'the no-rows run'

[ "$(t_failures)" -eq 0 ] || exit 1
printf 'test-adversarial-review-cycles.sh passed.\n'
